use std::collections::hash_map::Values;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::ProtestError;
use crate::{RequestStream, Response, ResponseSender, TRouter};
use tokio::select;
use tokio::time::interval;
use tokio_quiche::ServerH3Controller;
use tokio_quiche::http3::driver::{H3Event, OutboundFrameSender, ServerH3Event};
use tracing::{debug, error, info, trace, warn};

pub const MAX_IDLE_STREAM: std::time::Duration = std::time::Duration::from_secs(60);
pub const MAX_IDLE_CONNECTION: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Debug)]
struct HttpStreamManager {
    pub streams: HashMap<u64, HttpStream>,
    pub dispatched: HashSet<u64>,
    pub last_activity: Instant,
}

impl HttpStreamManager {
    fn new() -> Self {
        Self {
            streams: HashMap::default(),
            dispatched: HashSet::default(),
            last_activity: Instant::now(),
        }
    }

    pub fn add(&mut self, stream_id: u64, request: RequestStream, sender: OutboundFrameSender) {
        self.streams
            .insert(stream_id, HttpStream::new(stream_id, request, sender));
    }

    pub fn get(&mut self, stream_id: u64) -> Option<&HttpStream> {
        self.streams
            .get_mut(&stream_id)
            .into_iter()
            .for_each(HttpStream::extend_idle_time);
        self.streams.get(&stream_id)
    }

    pub fn streams(&self) -> Values<'_, u64, HttpStream> {
        self.streams.values()
    }

    pub fn contains(&self, stream_id: u64) -> bool {
        self.streams.contains_key(&stream_id)
    }

    pub fn is_dispatched(&self, stream_id: u64) -> bool {
        self.dispatched.contains(&stream_id)
    }

    pub fn clear(&mut self, stream_id: u64) {
        self.dispatched.remove(&stream_id);
        self.streams.remove(&stream_id);
    }

    pub fn dispatched(&mut self, stream_id: u64) -> Option<HttpStream> {
        self.dispatched.insert(stream_id);
        self.streams.remove(&stream_id)
    }

    pub fn is_connection_idle_too_long(&self) -> bool {
        Instant::now().duration_since(self.last_activity) > MAX_IDLE_CONNECTION
    }
}

#[derive(Debug)]
struct HttpStream {
    pub stream_id: u64,
    pub request: RequestStream,
    pub sender: OutboundFrameSender,
    pub idle_at: Instant,
}

impl HttpStream {
    pub fn new(stream_id: u64, request: RequestStream, sender: OutboundFrameSender) -> Self {
        Self {
            stream_id,
            request,
            sender,
            idle_at: Instant::now() + MAX_IDLE_STREAM,
        }
    }

    pub fn is_idle_too_long(&self) -> bool {
        self.idle_at > Instant::now()
    }

    pub fn extend_idle_time(&mut self) {
        self.idle_at = Instant::now() + MAX_IDLE_STREAM;
    }
}

#[allow(unused)]
pub async fn safely_handle_connection(
    controller: ServerH3Controller,
    routers: Arc<Vec<Box<dyn TRouter>>>,
) {
    let _ = handle_connection(controller, routers)
        .await
        .inspect_err(|err| error!(?err, "Error handling connection"));
}

#[allow(unused)]
pub async fn handle_connection(
    mut controller: ServerH3Controller,
    routers: Arc<Vec<Box<dyn TRouter>>>,
) -> Result<(), ProtestError> {
    let mut streams = HttpStreamManager::new();

    let receiver = controller.event_receiver_mut();
    let mut interval = interval(Duration::from_secs(1));

    while select! {
        Some(event) = receiver.recv() => handle_connection_event(event, &mut streams, routers.clone()).await,
        poll = interval.tick() => monitor_connection_tick(&mut streams).await
    }? { /* no-op */ }

    debug!("Connection finished");
    Ok(())
}

async fn handle_connection_event(
    event: ServerH3Event,
    streams: &mut HttpStreamManager,
    routers: Arc<Vec<Box<dyn TRouter>>>,
) -> Result<bool, ProtestError> {
    match event {
        ServerH3Event::Core(H3Event::IncomingSettings { settings }) => {
            trace!(?settings, "Http Event (Core)- Incoming settings");
        }
        ServerH3Event::Headers {
            incoming_headers,
            priority,
            is_in_early_data,
        } => {
            trace!(
                ?incoming_headers,
                ?priority,
                ?is_in_early_data,
                "Http Event (Server) - Incoming headers"
            );
            let stream_id = incoming_headers.stream_id;

            let read_fin = incoming_headers.read_fin;
            let req = RequestStream::try_from_incoming(incoming_headers)?;
            streams.add(stream_id, req.0, req.1);

            if read_fin {
                bg_dispatch_request(streams, routers.clone(), stream_id);
            }
        }
        ServerH3Event::Core(H3Event::BodyBytesReceived {
            stream_id,
            num_bytes,
            fin,
        }) => {
            trace!(
                ?stream_id,
                ?num_bytes,
                ?fin,
                "Http Event - Body Received (Core)"
            );
            let stream = streams.get(stream_id);

            match (stream.is_some(), fin) {
                (false, _) if streams.is_dispatched(stream_id) => {
                    trace!(
                        ?stream_id,
                        "Http Event (Core) - Ignoring trailing FIN after early dispatch"
                    );
                }
                (false, _) => Err("Request was consumed in error")?,
                (true, false) => trace!("Http Event (Core) - Receiving body bytes"),
                (true, true) => bg_dispatch_request(streams, routers.clone(), stream_id),
            };
        }
        ServerH3Event::Core(H3Event::IncomingHeaders(incoming_headers)) => {
            trace!(?incoming_headers, "Http Event (Core) - Incoming headers");
            match (
                streams.contains(incoming_headers.stream_id),
                incoming_headers.read_fin,
            ) {
                (false, _) => Err("Request was consumed in error")?,
                (true, false) => info!("More bytes available"),
                (true, true) => {
                    bg_dispatch_request(streams, routers.clone(), incoming_headers.stream_id)
                }
            }
        }
        ServerH3Event::Core(H3Event::StreamClosed { stream_id }) => {
            trace!(?stream_id, "Http Event (Core) - Stream closed");
            streams.clear(stream_id);
        }
        unhandled_event => {
            warn!(?unhandled_event, "Http Event - Unhandled");
        }
    };
    Ok(true)
}

fn bg_dispatch_request(
    streams: &mut HttpStreamManager,
    routers: Arc<Vec<Box<dyn TRouter>>>,
    stream_id: u64,
) {
    let request = streams.dispatched(stream_id).unwrap();
    tokio::spawn(async move {
        if let Err(err) = dispatch_request(request, &routers).await {
            error!(?err, "Http Event - Request Failed");
        }
    });
}

async fn dispatch_request(
    mut request: HttpStream,
    routers: &[Box<dyn TRouter>],
) -> Result<(), ProtestError> {
    trace!(?request, "Http Event (Core) - Responding to request");

    if let Some(router) = routers
        .iter()
        .find(|router| router.can_handle_request(&request.request))
    {
        router
            .handle_request(request.request, &mut request.sender)
            .await?;
        return Ok(());
    }

    Response::<String>::new(crate::Status::NotFound, "Route not found".into())
        .send(&mut request.sender)
        .await?;
    Ok(())
}

async fn monitor_connection_tick(streams: &mut HttpStreamManager) -> Result<bool, ProtestError> {
    if streams.is_connection_idle_too_long() {
        return Ok(false);
    }

    for stream_id in streams
        .streams()
        .filter(|v| v.is_idle_too_long())
        .map(|v| v.stream_id)
        .collect::<Vec<u64>>()
    {
        debug!(stream_id, "Http Monitor - Dropping idle stream");
        streams.clear(stream_id);
    }

    Ok(true)
}
