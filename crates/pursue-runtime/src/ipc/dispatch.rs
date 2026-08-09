//! Service dispatch: routes requests to registered handlers.

use pursue_core::{Error, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::protocol::{IpcError, IpcErrorCode, MethodName, Request, Response, ServiceId};

/// Handles methods for exactly one service.
pub trait Handler: Send {
    /// The service identity this handler serves.
    fn service_id(&self) -> &ServiceId;

    /// Handles a method invocation, returning a payload or a typed IPC error.
    ///
    /// Handlers own method-level dispatch (including returning
    /// [`IpcErrorCode::UnknownMethod`] for unknown methods).
    fn handle(
        &mut self,
        method: &MethodName,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError>;
}

/// Routes requests to handlers by service identity.
///
/// Unknown services receive an [`IpcErrorCode::UnknownService`] response;
/// everything else is delegated to the handler. Registration rejects
/// duplicate service identities.
#[derive(Default)]
pub struct Router {
    handlers: HashMap<ServiceId, Box<dyn Handler>>,
}

impl Router {
    /// Creates an empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler. Fails if a handler for the same service is
    /// already registered.
    pub fn register(&mut self, handler: Box<dyn Handler>) -> Result<()> {
        let id = handler.service_id().clone();
        if self.handlers.contains_key(&id) {
            return Err(Error::InvalidInput(format!(
                "handler already registered for service {id}"
            )));
        }
        self.handlers.insert(id, handler);
        Ok(())
    }

    /// Whether a handler is registered for `service`.
    pub fn has_service(&self, service: &ServiceId) -> bool {
        self.handlers.contains_key(service)
    }

    /// Dispatches a request and produces a response.
    pub fn handle(&mut self, request: &Request) -> Response {
        let Some(handler) = self.handlers.get_mut(&request.service) else {
            let error = IpcError::new(
                IpcErrorCode::UnknownService,
                &format!("unknown service: {}", request.service),
            )
            .expect("non-empty message");
            return Response::failure(request.id, error);
        };
        match handler.handle(&request.method, &request.params) {
            Ok(result) => Response::success(request.id, result),
            Err(error) => Response::failure(request.id, error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Handler, Router};
    use crate::ipc::protocol::{IpcError, IpcErrorCode, MethodName, Request, ServiceId};
    use pursue_core::Error;
    use serde_json::{Value, json};

    struct EchoHandler {
        id: ServiceId,
    }

    impl Handler for EchoHandler {
        fn service_id(&self) -> &ServiceId {
            &self.id
        }

        fn handle(
            &mut self,
            method: &MethodName,
            params: &Value,
        ) -> std::result::Result<Value, IpcError> {
            match method.as_str() {
                "echo" => Ok(params.clone()),
                _ => Err(IpcError::new(
                    IpcErrorCode::UnknownMethod,
                    &format!("no such method: {method}"),
                )
                .unwrap()),
            }
        }
    }

    struct FailingHandler {
        id: ServiceId,
    }

    impl Handler for FailingHandler {
        fn service_id(&self) -> &ServiceId {
            &self.id
        }

        fn handle(
            &mut self,
            _method: &MethodName,
            _params: &Value,
        ) -> std::result::Result<Value, IpcError> {
            Err(IpcError::new(IpcErrorCode::Internal, "handler exploded").unwrap())
        }
    }

    fn router_with_echo() -> Router {
        let mut router = Router::new();
        router
            .register(Box::new(EchoHandler {
                id: ServiceId::new("echo").unwrap(),
            }))
            .unwrap();
        router
    }

    #[test]
    fn routes_echo_request() {
        let mut router = router_with_echo();
        let request = Request::new(
            1,
            ServiceId::new("echo").unwrap(),
            MethodName::new("echo").unwrap(),
            json!({"k": "v"}),
        );
        let response = router.handle(&request);
        assert_eq!(response.id, 1);
        assert!(response.is_success());
        assert_eq!(response.result, Some(json!({"k": "v"})));
    }

    #[test]
    fn unknown_service_gets_typed_error_with_matching_id() {
        let mut router = router_with_echo();
        let request = Request::new(
            9,
            ServiceId::new("ghost").unwrap(),
            MethodName::new("echo").unwrap(),
            json!(null),
        );
        let response = router.handle(&request);
        assert_eq!(response.id, 9);
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code(), IpcErrorCode::UnknownService);
        assert!(error.message().contains("ghost"));
    }

    #[test]
    fn unknown_method_is_reported_by_handler() {
        let mut router = router_with_echo();
        let request = Request::new(
            2,
            ServiceId::new("echo").unwrap(),
            MethodName::new("nope").unwrap(),
            json!(null),
        );
        let response = router.handle(&request);
        assert_eq!(
            response.error.as_ref().unwrap().code(),
            IpcErrorCode::UnknownMethod
        );
    }

    #[test]
    fn handler_failure_propagates_as_typed_error() {
        let mut router = Router::new();
        router
            .register(Box::new(FailingHandler {
                id: ServiceId::new("fail").unwrap(),
            }))
            .unwrap();
        let request = Request::new(
            3,
            ServiceId::new("fail").unwrap(),
            MethodName::new("do_it").unwrap(),
            json!(null),
        );
        let response = router.handle(&request);
        assert_eq!(
            response.error.as_ref().unwrap().code(),
            IpcErrorCode::Internal
        );
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut router = router_with_echo();
        let duplicate = router.register(Box::new(EchoHandler {
            id: ServiceId::new("echo").unwrap(),
        }));
        let err = duplicate.unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
        assert!(router.has_service(&ServiceId::new("echo").unwrap()));
        assert!(!router.has_service(&ServiceId::new("other").unwrap()));
    }
}
