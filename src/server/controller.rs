use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use crate::ProtestError;
use crate::{RequestStream, Response, ResponseSender, TRouter};
use tokio_quiche::ServerH3Controller;
use tokio_quiche::http3::driver::{H3Event, OutboundFrameSender, ServerH3Event};
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Default)]
pub struct HttpStreamManager {
    pub requests: HashMap<u64, HttpStream>,
    pub dispatched: HashSet<u64>,
}

impl HttpStreamManager {
    pub fn add(&mut self, stream_id: u64, request: RequestStream, sender: OutboundFrameSender) {
        self.requests
            .insert(stream_id, HttpStream::new(stream_id, request, sender));
    }

    pub fn get(&mut self, stream_id: u64) -> Option<&HttpStream> {
        let stream = self.requests.get_mut(&stream_id);
        if stream.is_some() {
            stream.map(|s| s.extend_idle_time());
        }
        self.requests.get(&stream_id)
    }

    pub fn contains(&self, stream_id: u64) -> bool {
        self.requests.contains_key(&stream_id)
    }

    pub fn extend_idle_time(&mut self, stream_id: u64) {
        if let Some(stream) = self.requests.get_mut(&stream_id) {
            stream.extend_idle_time();
        }
    }

    pub fn is_expired(&self, stream_id: u64) -> bool {
        if let Some(stream) = self.requests.get(&stream_id) {
            stream.is_expired()
        } else {
            false
        }
    }

    pub fn is_dispatched(&self, stream_id: u64) -> bool {
        self.dispatched.contains(&stream_id)
    }

    pub fn clear(&mut self, stream_id: u64) {
        self.dispatched.remove(&stream_id);
        self.requests.remove(&stream_id);
    }

    pub fn dispatched(&mut self, stream_id: u64) -> Option<HttpStream> {
        self.dispatched.insert(stream_id);
        self.requests.remove(&stream_id)
    }
}

#[derive(Debug)]
pub struct HttpStream {
    pub stream_id: u64,
    pub request: RequestStream,
    pub sender: OutboundFrameSender,
    pub start: Instant,
    pub idle_at: Instant,
}

impl HttpStream {
    const MAX_IDLE_TIME: std::time::Duration = std::time::Duration::from_secs(60);
    pub fn new(stream_id: u64, request: RequestStream, sender: OutboundFrameSender) -> Self {
        Self {
            stream_id,
            request,
            sender,
            start: Instant::now(),
            idle_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.idle_at > Instant::now()
    }

    pub fn extend_idle_time(&mut self) {
        self.idle_at = Instant::now() + Self::MAX_IDLE_TIME;
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
    let mut streams = HttpStreamManager::default();

    while let Some(event) = controller.event_receiver_mut().recv().await {
        match event {
            ServerH3Event::Core(H3Event::IncomingSettings { settings }) => {
                trace!(?settings, "Http Event (Core)- Incoming settings");
            }
            ServerH3Event::Headers {
                mut incoming_headers,
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
                    bg_dispatch_request(&mut streams, routers.clone(), stream_id);
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
                    (true, true) => bg_dispatch_request(&mut streams, routers.clone(), stream_id),
                };
            }
            ServerH3Event::Core(H3Event::IncomingHeaders(mut incoming_headers)) => {
                trace!(?incoming_headers, "Http Event (Core) - Incoming headers");
                match (
                    streams.contains(incoming_headers.stream_id),
                    incoming_headers.read_fin,
                ) {
                    (false, _) => Err("Request was consumed in error")?,
                    (true, false) => info!("More bytes available"),
                    (true, true) => bg_dispatch_request(
                        &mut streams,
                        routers.clone(),
                        incoming_headers.stream_id,
                    ),
                }
            }
            ServerH3Event::Core(H3Event::StreamClosed { stream_id }) => {
                trace!(?stream_id, "Http Event (Core) - Stream closed");
                streams.clear(stream_id);
            }
            unhandled_event => {
                warn!(?unhandled_event, "Http Event - Unhandled");
            }
        }
    }
    debug!("Connection finished");
    Ok(())
}

fn bg_dispatch_request(
    streams: &mut HttpStreamManager,
    routers: Arc<Vec<Box<dyn TRouter>>>,
    stream_id: u64,
) {
    let request = streams.dispatched(stream_id).unwrap();
    tokio::spawn(async move {
        if let Err(err) = dispatch_request(request, &*routers).await {
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
