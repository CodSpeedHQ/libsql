// This file is @generated by prost-build.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchDatabaseRequest {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DatabaseChunk {
    #[prost(bytes = "bytes", tag = "1")]
    pub data: ::prost::bytes::Bytes,
    #[prost(uint64, tag = "2")]
    pub offset: u64,
    #[prost(bool, tag = "3")]
    pub is_last_chunk: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PullWalRequest {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    pub client_last_checkpoint_frame_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PullWalResponse {
    #[prost(message, repeated, tag = "1")]
    pub wal: ::prost::alloc::vec::Vec<WalFrame>,
    #[prost(uint64, tag = "2")]
    pub server_last_checkpoint_frame_id: u64,
    #[prost(bool, tag = "3")]
    pub need_full_db_sync: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WalFrame {
    #[prost(uint64, tag = "1")]
    pub frame_id: u64,
    #[prost(bytes = "bytes", tag = "2")]
    pub data: ::prost::bytes::Bytes,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushWalRequest {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    pub base_frame_id: u64,
    #[prost(message, repeated, tag = "3")]
    pub new_frames: ::prost::alloc::vec::Vec<WalFrame>,
    #[prost(uint64, tag = "4")]
    pub last_checkpoint_frame_id: u64,
    #[prost(bool, tag = "5")]
    pub request_checkpoint: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushWalResponse {
    #[prost(enumeration = "push_wal_response::Status", tag = "1")]
    pub status: i32,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub server_wal: ::prost::alloc::vec::Vec<WalFrame>,
    #[prost(uint64, tag = "4")]
    pub server_last_checkpoint_frame_id: u64,
    #[prost(bool, tag = "5")]
    pub perform_checkpoint: bool,
    #[prost(uint64, tag = "6")]
    pub checkpoint_frame_id: u64,
}
/// Nested message and enum types in `PushWALResponse`.
pub mod push_wal_response {
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Status {
        Success = 0,
        Conflict = 1,
        Error = 2,
        NeedFullSync = 3,
    }
    impl Status {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Status::Success => "SUCCESS",
                Status::Conflict => "CONFLICT",
                Status::Error => "ERROR",
                Status::NeedFullSync => "NEED_FULL_SYNC",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "SUCCESS" => Some(Self::Success),
                "CONFLICT" => Some(Self::Conflict),
                "ERROR" => Some(Self::Error),
                "NEED_FULL_SYNC" => Some(Self::NeedFullSync),
                _ => None,
            }
        }
    }
}
/// Generated client implementations.
pub mod wal_sync_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct WalSyncClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl WalSyncClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> WalSyncClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> WalSyncClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            WalSyncClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Fetch the database file
        pub async fn fetch_database(
            &mut self,
            request: impl tonic::IntoRequest<super::FetchDatabaseRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::DatabaseChunk>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/walsync.WALSync/FetchDatabase",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("walsync.WALSync", "FetchDatabase"));
            self.inner.server_streaming(req, path, codec).await
        }
        /// Pull the WAL from the server
        pub async fn pull_wal(
            &mut self,
            request: impl tonic::IntoRequest<super::PullWalRequest>,
        ) -> std::result::Result<
            tonic::Response<super::PullWalResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/walsync.WALSync/PullWAL");
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new("walsync.WALSync", "PullWAL"));
            self.inner.unary(req, path, codec).await
        }
        /// Push local changes to the server and potentially trigger checkpointing
        pub async fn push_wal(
            &mut self,
            request: impl tonic::IntoRequest<super::PushWalRequest>,
        ) -> std::result::Result<
            tonic::Response<super::PushWalResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/walsync.WALSync/PushWAL");
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new("walsync.WALSync", "PushWAL"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod wal_sync_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with WalSyncServer.
    #[async_trait]
    pub trait WalSync: Send + Sync + 'static {
        /// Server streaming response type for the FetchDatabase method.
        type FetchDatabaseStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<super::DatabaseChunk, tonic::Status>,
            >
            + Send
            + 'static;
        /// Fetch the database file
        async fn fetch_database(
            &self,
            request: tonic::Request<super::FetchDatabaseRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::FetchDatabaseStream>,
            tonic::Status,
        >;
        /// Pull the WAL from the server
        async fn pull_wal(
            &self,
            request: tonic::Request<super::PullWalRequest>,
        ) -> std::result::Result<tonic::Response<super::PullWalResponse>, tonic::Status>;
        /// Push local changes to the server and potentially trigger checkpointing
        async fn push_wal(
            &self,
            request: tonic::Request<super::PushWalRequest>,
        ) -> std::result::Result<tonic::Response<super::PushWalResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct WalSyncServer<T: WalSync> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: WalSync> WalSyncServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for WalSyncServer<T>
    where
        T: WalSync,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/walsync.WALSync/FetchDatabase" => {
                    #[allow(non_camel_case_types)]
                    struct FetchDatabaseSvc<T: WalSync>(pub Arc<T>);
                    impl<
                        T: WalSync,
                    > tonic::server::ServerStreamingService<super::FetchDatabaseRequest>
                    for FetchDatabaseSvc<T> {
                        type Response = super::DatabaseChunk;
                        type ResponseStream = T::FetchDatabaseStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FetchDatabaseRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WalSync>::fetch_database(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchDatabaseSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/walsync.WALSync/PullWAL" => {
                    #[allow(non_camel_case_types)]
                    struct PullWALSvc<T: WalSync>(pub Arc<T>);
                    impl<T: WalSync> tonic::server::UnaryService<super::PullWalRequest>
                    for PullWALSvc<T> {
                        type Response = super::PullWalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PullWalRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WalSync>::pull_wal(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PullWALSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/walsync.WALSync/PushWAL" => {
                    #[allow(non_camel_case_types)]
                    struct PushWALSvc<T: WalSync>(pub Arc<T>);
                    impl<T: WalSync> tonic::server::UnaryService<super::PushWalRequest>
                    for PushWALSvc<T> {
                        type Response = super::PushWalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PushWalRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WalSync>::push_wal(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PushWALSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: WalSync> Clone for WalSyncServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: WalSync> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: WalSync> tonic::server::NamedService for WalSyncServer<T> {
        const NAME: &'static str = "walsync.WALSync";
    }
}