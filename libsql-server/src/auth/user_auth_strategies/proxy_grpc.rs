use crate::auth::{AuthError, Authenticated};

use super::{UserAuthContext, UserAuthStrategy};

pub struct ProxyGrpc {}

impl UserAuthStrategy for ProxyGrpc {
    fn authenticate(&self, ctx: UserAuthContext) -> Result<Authenticated, AuthError> {
        tracing::trace!("executing proxy grpc auth");
        let auth_str = None
            .or_else(|| ctx.custom_fields.get("proxy-authorization"))
            .or_else(|| ctx.custom_fields.get("x-proxy-authorization"));
    }

    fn required_fields(&self) -> Vec<String> {
        vec![
            "authorization".to_string(),
            "x-proxy-authorization".to_string(),
        ]
    }
}

impl ProxyGrpc {
    pub fn new() -> Self {
        Self {}
    }
}

// todo tests
