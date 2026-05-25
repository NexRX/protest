#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    strum::AsRefStr,
    strum::Display,
    strum::EnumIter,
    strum::IntoStaticStr,
)]
pub enum Status {
    // ---------- Informational Responses ----------
    /// 100 This interim response indicates that the client should continue the request or ignore the response if the request is already finished.
    Continue = 100,
    /// 101 This code is sent in response to an Upgrade request header from the client and indicates the protocol the server is switching to.
    SwitchingProtocols = 101,
    /// 102 This code was used in WebDAV contexts to indicate that a request has been received by the server, but no status was available at the time of the response.
    Processing = 102,
    /// 103 This status code is primarily intended to be used with the Link header, letting the user agent start preloading resources while the server prepares a response or preconnect to an origin from which the page will need resources.
    EarlyHints = 103,

    // ---------- Successful Responses ----------
    /// 200 The request succeeded. The result and meaning of "success" depends on the HTTP method:
    /// - `GET`: The resource has been fetched and transmitted in the message body.
    /// - `HEAD`: Representation headers are included in the response without any message body.
    /// - `PUT` or `POST`: The resource describing the result of the action is transmitted in the message body.
    /// - `TRACE`: The message body contains the request as received by the server.
    OK = 200,
    /// 201 The request succeeded, and a new resource was created as a result. This is typically the response sent after POST requests, or some PUT requests.
    Created = 201,
    /// 202 The request has been received but not yet acted upon. It is noncommittal, since there is no way in HTTP to later send an asynchronous response indicating the outcome of the request. It is intended for cases where another process or server handles the request, or for batch processing.
    Accepted = 202,
    /// 203 This response code means the returned metadata is not exactly the same as is available from the origin server, but is collected from a local or a third-party copy. This is mostly used for mirrors or backups of another resource. Except for that specific case, the 200 OK response is preferred to this status.
    NonAuthoritativeInformation = 203,
    /// 204 There is no content to send for this request, but the headers are useful. The user agent may update its cached headers for this resource with the new ones.
    NoContent = 204,
    /// 205 Tells the user agent to reset the document which sent this request.
    ResetContent = 205,
    /// 206 This response code is used in response to a range request when the client has requested a part or parts of a resource.
    PartialContent = 206,
    /// 207 Conveys information about multiple resources, for situations where multiple status codes might be appropriate. (WebDAV)
    MultiStatus = 207,
    /// 208 Used inside a `<dav:propstat>` response element to avoid repeatedly enumerating the internal members of multiple bindings to the same collection. (WebDAV)
    AlreadyReported = 208,
    /// 226 The server has fulfilled a GET request for the resource, and the response is a representation of the result of one or more instance-manipulations applied to the current instance. (HTTP Delta encoding)
    IMUsed = 226,

    // ---------- Redirection Messages ----------
    /// 300 In agent-driven content negotiation, the request has more than one possible response and the user agent or user should choose one of them. There is no standardized way for clients to automatically choose one of the responses, so this is rarely used.
    MultipleChoices = 300,
    /// 301 The URL of the requested resource has been changed permanently. The new URL is given in the response.
    MovedPermanently = 301,
    /// 302 This response code means that the URI of requested resource has been changed temporarily. Further changes in the URI might be made in the future, so the same URI should be used by the client in future requests.
    Found = 302,
    /// 303 The server sent this response to direct the client to get the requested resource at another URI with a GET request.
    SeeOther = 303,
    /// 304 This is used for caching purposes. It tells the client that the response has not been modified, so the client can continue to use the same cached version of the response.
    NotModified = 304,
    /// 305 Defined in a previous version of the HTTP specification to indicate that a requested response must be accessed by a proxy. It has been deprecated due to security concerns regarding in-band configuration of a proxy.
    UseProxy = 305,
    /// 306 This response code is no longer used; but is reserved. It was used in a previous version of the HTTP/1.1 specification.
    Unused = 306,
    /// 307 The server sends this response to direct the client to get the requested resource at another URI with the same method that was used in the prior request. This has the same semantics as the 302 Found response code, with the exception that the user agent must not change the HTTP method used: if a POST was used in the first request, a POST must be used in the redirected request.
    TemporaryRedirect = 307,
    /// 308 This means that the resource is now permanently located at another URI, specified by the Location response header. This has the same semantics as the 301 Moved Permanently HTTP response code, with the exception that the user agent must not change the HTTP method used: if a POST was used in the first request, a POST must be used in the second request.
    PermanentRedirect = 308,

    // ---------- Client error responses ----------
    /// 400 The server cannot or will not process the request due to something that is perceived to be a client error (e.g., malformed request syntax, invalid request message framing, or deceptive request routing).
    BadRequest = 400,
    /// 401 Although the HTTP standard specifies "unauthorized", semantically this response means "unauthenticated". That is, the client must authenticate itself to get the requested response.
    Unauthorized = 401,
    /// 402 The initial purpose of this code was for digital payment systems, however this status code is rarely used and no standard convention exists.
    PaymentRequired = 402,
    /// 403 The client does not have access rights to the content; that is, it is unauthorized, so the server is refusing to give the requested resource. Unlike 401 Unauthorized, the client's identity is known to the server.
    Forbidden = 403,
    /// 404 The server cannot find the requested resource. In the browser, this means the URL is not recognized. In an API, this can also mean that the endpoint is valid but the resource itself does not exist. Servers may also send this response instead of 403 Forbidden to hide the existence of a resource from an unauthorized client. This response code is probably the most well known due to its frequent occurrence on the web.
    NotFound = 404,
    /// 405 The request method is known by the server but is not supported by the target resource. For example, an API may not allow DELETE on a resource, or the TRACE method entirely.
    MethodNotAllowed = 405,
    /// 406 This response is sent when the web server, after performing server-driven content negotiation, doesn't find any content that conforms to the criteria given by the user agent.
    NotAcceptable = 406,
    /// 407 This is similar to 401 Unauthorized but authentication is needed to be done by a proxy.
    ProxyAuthenticationRequired = 407,
    /// 408 This response is sent on an idle connection by some servers, even without any previous request by the client. It means that the server would like to shut down this unused connection. This response is used much more since some browsers use HTTP pre-connection mechanisms to speed up browsing. Some servers may shut down a connection without sending this message.
    RequestTimeout = 408,
    /// 409 This response is sent when a request conflicts with the current state of the server. In WebDAV remote web authoring, 409 responses are errors sent to the client so that a user might be able to resolve a conflict and resubmit the request.
    Conflict = 409,
    /// 410 This response is sent when the requested content has been permanently deleted from server, with no forwarding address. Clients are expected to remove their caches and links to the resource. The HTTP specification intends this status code to be used for "limited-time, promotional services". APIs should not feel compelled to indicate resources that have been deleted with this status code.
    Gone = 410,
    /// 411 Server rejected the request because the Content-Length header field is not defined and the server requires it.
    LengthRequired = 411,
    /// 412 In conditional requests, the client has indicated preconditions in its headers which the server does not meet.
    PreconditionFailed = 412,
    /// 413 The request body is larger than limits defined by server. The server might close the connection or return a Retry-After header field.
    ContentTooLarge = 413,
    /// 414 The URI requested by the client is longer than the server is willing to interpret.
    URITooLong = 414,
    /// 415 The media format of the requested data is not supported by the server, so the server is rejecting the request.
    UnsupportedMediaType = 415,
    /// 416 The ranges specified by the Range header field in the request cannot be fulfilled. It's possible that the range is outside the size of the target resource's data.
    RangeNotSatisfiable = 416,
    /// 417 This response code means the expectation indicated by the Expect request header field cannot be met by the server.
    ExpectationFailed = 417,
    /// 418 The server refuses the attempt to brew coffee with a teapot.
    ImATeapot = 418,
    /// 421 The request was directed at a server that is not able to produce a response. This can be sent by a server that is not configured to produce responses for the combination of scheme and authority that are included in the request URI.
    MisdirectedRequest = 421,
    /// 422 The request was well-formed but was unable to be followed due to semantic errors. (WebDAV)
    UnprocessableContent = 422,
    /// 423 The resource that is being accessed is locked. (WebDAV)
    Locked = 423,
    /// 424 The request failed due to failure of a previous request. (WebDAV)
    FailedDependency = 424,
    /// 425 Indicates that the server is unwilling to risk processing a request that might be replayed.
    TooEarly = 425,
    /// 426 The server refuses to perform the request using the current protocol but might be willing to do so after the client upgrades to a different protocol. The server sends an Upgrade header in a 426 response to indicate the required protocol(s).
    UpgradeRequired = 426,
    /// 428 The origin server requires the request to be conditional. This response is intended to prevent the 'lost update' problem, where a client GETs a resource's state, modifies it and PUTs it back to the server, when meanwhile a third party has modified the state on the server, leading to a conflict.
    PreconditionRequired = 428,
    /// 429 The user has sent too many requests in a given amount of time (rate limiting).
    TooManyRequests = 429,
    /// 431 The server is unwilling to process the request because its header fields are too large. The request may be resubmitted after reducing the size of the request header fields.
    RequestHeaderFieldsTooLarge = 431,
    /// 451 The user agent requested a resource that cannot legally be provided, such as a web page censored by a government.
    UnavailableForLegalReasons = 451,

    // ---------- Server error responses ----------
    /// 500 The server has encountered a situation it does not know how to handle. This error is generic, indicating that the server cannot find a more appropriate 5XX status code to respond with.
    #[default]
    InternalServerError = 500,
    /// 501 The request method is not supported by the server and cannot be handled. The only methods that servers are required to support (and therefore must not return this code) are GET and HEAD.
    NotImplemented = 501,
    /// 502 This error response means that the server, while working as a gateway to get a response needed to handle the request, got an invalid response.
    BadGateway = 502,
    /// 503 The server is not ready to handle the request. Common causes are a server that is down for maintenance or that is overloaded. Note that together with this response, a user-friendly page explaining the problem should be sent. This response should be used for temporary conditions and the Retry-After HTTP header should, if possible, contain the estimated time before the recovery of the service. The webmaster must also take care about the caching-related headers that are sent along with this response, as these temporary condition responses should usually not be cached.
    ServiceUnavailable = 503,
    /// 504 This error response is given when the server is acting as a gateway and cannot get a response in time.
    GatewayTimeout = 504,
    /// 505 The HTTP version used in the request is not supported by the server.
    HTTPVersionNotSupported = 505,
    /// 506 The server has an internal configuration error: during content negotiation, the chosen variant is configured to engage in content negotiation itself, which results in circular references when creating responses.
    VariantAlsoNegotiates = 506,
    /// 507 The method could not be performed on the resource because the server is unable to store the representation needed to successfully complete the request. (WebDAV)
    InsufficientStorage = 507,
    /// 508 The server detected an infinite loop while processing the request. (WebDAV)
    LoopDetected = 508,
    /// 510 The client request declares an HTTP Extension (RFC 2774) that should be used to process the request, but the extension is not supported.
    NotExtended = 510,
    /// 511 Indicates that the client needs to authenticate to gain network access.
    NetworkAuthenticationRequired = 511,
}

impl Status {
    const MIN: u16 = 100;
    const MAX: u16 = 511;

    pub fn code(&self) -> u16 {
        *self as u16
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.code().to_string().into_bytes()
    }
}

impl TryFrom<u16> for Status {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            100 => Ok(Self::Continue),
            101 => Ok(Self::SwitchingProtocols),
            102 => Ok(Self::Processing),
            103 => Ok(Self::EarlyHints),
            200 => Ok(Self::OK),
            201 => Ok(Self::Created),
            202 => Ok(Self::Accepted),
            203 => Ok(Self::NonAuthoritativeInformation),
            204 => Ok(Self::NoContent),
            205 => Ok(Self::ResetContent),
            206 => Ok(Self::PartialContent),
            207 => Ok(Self::MultiStatus),
            208 => Ok(Self::AlreadyReported),
            226 => Ok(Self::IMUsed),
            300 => Ok(Self::MultipleChoices),
            301 => Ok(Self::MovedPermanently),
            302 => Ok(Self::Found),
            303 => Ok(Self::SeeOther),
            304 => Ok(Self::NotModified),
            305 => Ok(Self::UseProxy),
            306 => Ok(Self::Unused),
            307 => Ok(Self::TemporaryRedirect),
            308 => Ok(Self::PermanentRedirect),
            400 => Ok(Self::BadRequest),
            401 => Ok(Self::Unauthorized),
            402 => Ok(Self::PaymentRequired),
            403 => Ok(Self::Forbidden),
            404 => Ok(Self::NotFound),
            405 => Ok(Self::MethodNotAllowed),
            406 => Ok(Self::NotAcceptable),
            407 => Ok(Self::ProxyAuthenticationRequired),
            408 => Ok(Self::RequestTimeout),
            409 => Ok(Self::Conflict),
            410 => Ok(Self::Gone),
            411 => Ok(Self::LengthRequired),
            412 => Ok(Self::PreconditionFailed),
            413 => Ok(Self::ContentTooLarge),
            414 => Ok(Self::URITooLong),
            415 => Ok(Self::UnsupportedMediaType),
            416 => Ok(Self::RangeNotSatisfiable),
            417 => Ok(Self::ExpectationFailed),
            418 => Ok(Self::ImATeapot),
            421 => Ok(Self::MisdirectedRequest),
            422 => Ok(Self::UnprocessableContent),
            423 => Ok(Self::Locked),
            424 => Ok(Self::FailedDependency),
            425 => Ok(Self::TooEarly),
            426 => Ok(Self::UpgradeRequired),
            428 => Ok(Self::PreconditionRequired),
            429 => Ok(Self::TooManyRequests),
            431 => Ok(Self::RequestHeaderFieldsTooLarge),
            451 => Ok(Self::UnavailableForLegalReasons),
            500 => Ok(Self::InternalServerError),
            501 => Ok(Self::NotImplemented),
            502 => Ok(Self::BadGateway),
            503 => Ok(Self::ServiceUnavailable),
            504 => Ok(Self::GatewayTimeout),
            505 => Ok(Self::HTTPVersionNotSupported),
            506 => Ok(Self::VariantAlsoNegotiates),
            507 => Ok(Self::InsufficientStorage),
            508 => Ok(Self::LoopDetected),
            510 => Ok(Self::NotExtended),
            511 => Ok(Self::NetworkAuthenticationRequired),
            _ => Err(()),
        }
    }
}

macro_rules! try_from_number {
    ($target_ty:ty) => {
        impl TryFrom<$target_ty> for Status {
            type Error = ();

            fn try_from(value: $target_ty) -> Result<Self, Self::Error> {
                if value > Self::MAX as $target_ty || value < Self::MIN as $target_ty {
                    return Err(());
                }

                Self::try_from(value as u16)
            }
        }
    };
}

try_from_number!(u32);
try_from_number!(u64);
try_from_number!(u128);
try_from_number!(usize);
try_from_number!(i16);
try_from_number!(i32);
try_from_number!(i64);
try_from_number!(i128);
try_from_number!(isize);

impl From<Status> for u16 {
    fn from(value: Status) -> Self {
        value.code()
    }
}

macro_rules! into_number {
    ($target_ty:ty) => {
        impl From<Status> for $target_ty {
            fn from(value: Status) -> Self {
                value.code() as $target_ty
            }
        }
    };
}

into_number!(u32);
into_number!(u64);
into_number!(u128);
into_number!(usize);
into_number!(i16);
into_number!(i32);
into_number!(i64);
into_number!(i128);
into_number!(isize);

#[macro_export]
macro_rules! status {
    (100) => {
        $crate::response::Status::Continue
    };
    (101) => {
        $crate::response::Status::SwitchingProtocols
    };
    (102) => {
        $crate::response::Status::Processing
    };
    (103) => {
        $crate::response::Status::EarlyHints
    };
    (200) => {
        $crate::response::Status::OK
    };
    (201) => {
        $crate::response::Status::Created
    };
    (202) => {
        $crate::response::Status::Accepted
    };
    (203) => {
        $crate::response::Status::NonAuthoritativeInformation
    };
    (204) => {
        $crate::response::Status::NoContent
    };
    (205) => {
        $crate::response::Status::ResetContent
    };
    (206) => {
        $crate::response::Status::PartialContent
    };
    (207) => {
        $crate::response::Status::MultiStatus
    };
    (208) => {
        $crate::response::Status::AlreadyReported
    };
    (226) => {
        $crate::response::Status::IMUsed
    };
    (300) => {
        $crate::response::Status::MultipleChoices
    };
    (301) => {
        $crate::response::Status::MovedPermanently
    };
    (302) => {
        $crate::response::Status::Found
    };
    (303) => {
        $crate::response::Status::SeeOther
    };
    (304) => {
        $crate::response::Status::NotModified
    };
    (305) => {
        $crate::response::Status::UseProxy
    };
    (306) => {
        $crate::response::Status::Unused
    };
    (307) => {
        $crate::response::Status::TemporaryRedirect
    };
    (308) => {
        $crate::response::Status::PermanentRedirect
    };
    (400) => {
        $crate::response::Status::BadRequest
    };
    (401) => {
        $crate::response::Status::Unauthorized
    };
    (402) => {
        $crate::response::Status::PaymentRequired
    };
    (403) => {
        $crate::response::Status::Forbidden
    };
    (404) => {
        $crate::response::Status::NotFound
    };
    (405) => {
        $crate::response::Status::MethodNotAllowed
    };
    (406) => {
        $crate::response::Status::NotAcceptable
    };
    (407) => {
        $crate::response::Status::ProxyAuthenticationRequired
    };
    (408) => {
        $crate::response::Status::RequestTimeout
    };
    (409) => {
        $crate::response::Status::Conflict
    };
    (410) => {
        $crate::response::Status::Gone
    };
    (411) => {
        $crate::response::Status::LengthRequired
    };
    (412) => {
        $crate::response::Status::PreconditionFailed
    };
    (413) => {
        $crate::response::Status::ContentTooLarge
    };
    (414) => {
        $crate::response::Status::URITooLong
    };
    (415) => {
        $crate::response::Status::UnsupportedMediaType
    };
    (416) => {
        $crate::response::Status::RangeNotSatisfiable
    };
    (417) => {
        $crate::response::Status::ExpectationFailed
    };
    (418) => {
        $crate::response::Status::ImATeapot
    };
    (421) => {
        $crate::response::Status::MisdirectedRequest
    };
    (422) => {
        $crate::response::Status::UnprocessableContent
    };
    (423) => {
        $crate::response::Status::Locked
    };
    (424) => {
        $crate::response::Status::FailedDependency
    };
    (425) => {
        $crate::response::Status::TooEarly
    };
    (426) => {
        $crate::response::Status::UpgradeRequired
    };
    (428) => {
        $crate::response::Status::PreconditionRequired
    };
    (429) => {
        $crate::response::Status::TooManyRequests
    };
    (431) => {
        $crate::response::Status::RequestHeaderFieldsTooLarge
    };
    (451) => {
        $crate::response::Status::UnavailableForLegalReasons
    };
    (500) => {
        $crate::response::Status::InternalServerError
    };
    (501) => {
        $crate::response::Status::NotImplemented
    };
    (502) => {
        $crate::response::Status::BadGateway
    };
    (503) => {
        $crate::response::Status::ServiceUnavailable
    };
    (504) => {
        $crate::response::Status::GatewayTimeout
    };
    (505) => {
        $crate::response::Status::HTTPVersionNotSupported
    };
    (506) => {
        $crate::response::Status::VariantAlsoNegotiates
    };
    (507) => {
        $crate::response::Status::InsufficientStorage
    };
    (508) => {
        $crate::response::Status::LoopDetected
    };
    (510) => {
        $crate::response::Status::NotExtended
    };
    (511) => {
        $crate::response::Status::NetworkAuthenticationRequired
    };
}
