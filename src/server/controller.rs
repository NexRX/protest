use std::sync::Arc;

use crate::ProtestError;
use crate::{RequestStream, Response, ResponseSender, TRouter};
use tokio_quiche::ServerH3Controller;
use tokio_quiche::http3::driver::{H3Event, OutboundFrameSender, ServerH3Event};
use tracing::{debug, error, info, trace, warn};

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
    let routers = routers.as_slice();
    let mut request: Option<(RequestStream, OutboundFrameSender)> = None;
    let mut dispatched = false;

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
                assert!(
                    request.is_none(),
                    "Request shouldn't have more than one headers event"
                );

                let read_fin = incoming_headers.read_fin;
                request = Some(RequestStream::try_from_incoming(incoming_headers)?);

                // If the headers frame carried FIN the request has no body;
                // dispatch it immediately instead of waiting for body events.
                if read_fin {
                    handle_request_via_iter(request.take().unwrap(), routers).await?;
                    dispatched = true;
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

                match (request.is_some(), fin) {
                    (false, _) if dispatched => {
                        // Trailing FIN event after the request was already
                        // dispatched early via read_fin in the headers frame.
                        trace!(
                            ?stream_id,
                            "Http Event (Core) - Ignoring trailing FIN after early dispatch"
                        );
                    }
                    (false, _) => Err("Request was consumed in error")?,
                    (true, false) => trace!("Http Event (Core) - Receiving body bytes"),
                    (true, true) => {
                        dispatched = true;
                        handle_request_via_iter(request.take().unwrap(), routers).await?;
                    }
                };
            }
            ServerH3Event::Core(H3Event::IncomingHeaders(mut incoming_headers)) => {
                trace!(?incoming_headers, "Http Event (Core) - Incoming headers");
                match (request.is_some(), incoming_headers.read_fin) {
                    (false, _) => Err("Request was consumed in error")?,
                    (true, false) => info!("More bytes available"),
                    (true, true) => {
                        handle_request_via_iter(request.take().unwrap(), routers).await?;
                    }
                }
            }
            ServerH3Event::Core(H3Event::StreamClosed { stream_id }) => {
                trace!(?stream_id, "Http Event (Core) - Stream closed");
            }
            unhandled_event => {
                warn!(?unhandled_event, "Http Event - Unhandled");
            }
        }
    }
    debug!("Connection finished");
    Ok(())
}

async fn handle_request_via_iter(
    (request, mut send): (RequestStream, OutboundFrameSender),
    routers: &[Box<dyn TRouter>],
) -> Result<(), ProtestError> {
    trace!(?request, "Http Event (Core) - Responding to request");

    if let Some(router) = routers
        .iter()
        .find(|router| router.can_handle_request(&request))
    {
        router.handle_request(request, &mut send).await?;
        return Ok(());
    }

    Response::<String>::new(crate::Status::NotFound, "Route not found".into())
        .send(&mut send)
        .await?;
    Ok(())
}
