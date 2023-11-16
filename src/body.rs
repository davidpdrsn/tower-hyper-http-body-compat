use http_1::HeaderMap;
use http_body_1::Frame;
use pin_project_lite::pin_project;
use std::str::FromStr;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

// --- http-body 0.4 to http-body 1.0 ---

pin_project! {
    /// Converts an [http-body 0.4 `Body`] to an [http-body 1.0 `Body`].
    ///
    /// [http-body 0.4 `Body`]: https://docs.rs/http-body/latest/http_body/trait.Body.html
    /// [http-body 1.0 `Body`]: https://docs.rs/http-body/1.0.0-rc.2/http_body/trait.Body.html
    #[derive(Debug, Clone, Copy)]
    pub struct HttpBody04ToHttpBody1<B> {
        #[pin]
        body: B,
    }
}

impl<B> HttpBody04ToHttpBody1<B> {
    /// Create a new `HttpBody04ToHttpBody1`.
    #[inline]
    pub fn new(body: B) -> Self {
        Self { body }
    }
}

impl<B> http_body_1::Body for HttpBody04ToHttpBody1<B>
where
    B: http_body_04::Body,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.as_mut().project().body.poll_data(cx) {
            Poll::Ready(Some(Ok(buf))) => return Poll::Ready(Some(Ok(Frame::data(buf)))),
            Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => {}
            Poll::Pending => return Poll::Pending,
        }

        match self.as_mut().project().body.poll_trailers(cx) {
            Poll::Ready(Ok(Some(trailers))) => Poll::Ready(Some(Ok(Frame::trailers(
                http02_headermap_to_http1(trailers),
            )))),
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> http_body_1::SizeHint {
        let size_hint = self.body.size_hint();
        let mut out = http_body_1::SizeHint::new();
        out.set_lower(size_hint.lower());
        if let Some(upper) = size_hint.upper() {
            out.set_upper(upper);
        }
        out
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }
}

// --- http-body 1.0 to http-body 0.4 ---

pin_project! {
    /// Converts an [http-body 1.0 `Body`] to an [http-body 0.4 `Body`].
    ///
    /// [http-body 0.4 `Body`]: https://docs.rs/http-body/latest/http_body/trait.Body.html
    /// [http-body 1.0 `Body`]: https://docs.rs/http-body/1.0.0-rc.2/http_body/trait.Body.html
    #[derive(Debug, Clone, Default)]
    pub struct HttpBody1ToHttpBody04<B> {
        #[pin]
        body: B,
        trailers: Option<HeaderMap>,
    }
}

impl<B> HttpBody1ToHttpBody04<B> {
    /// Create a new `HttpBody1ToHttpBody04`.
    #[inline]
    pub fn new(body: B) -> Self {
        Self {
            body,
            trailers: None,
        }
    }
}

impl<B> http_body_04::Body for HttpBody1ToHttpBody04<B>
where
    B: http_body_1::Body,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.project();
        match ready!(this.body.poll_frame(cx)) {
            Some(Ok(frame)) => {
                let frame = match frame.into_data() {
                    Ok(data) => return Poll::Ready(Some(Ok(data))),
                    Err(frame) => frame,
                };

                match frame.into_trailers() {
                    Ok(trailers) => {
                        *this.trailers = Some(trailers);
                    }
                    Err(_frame) => {}
                }

                Poll::Ready(None)
            }
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
    }

    fn poll_trailers(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<http_02::HeaderMap>, Self::Error>> {
        loop {
            let this = self.as_mut().project();

            if let Some(trailers) = this.trailers.take() {
                break Poll::Ready(Ok(Some(http1_headermap_to_http02(trailers))));
            }

            match ready!(this.body.poll_frame(cx)) {
                Some(Ok(frame)) => match frame.into_trailers() {
                    Ok(trailers) => {
                        break Poll::Ready(Ok(Some(http1_headermap_to_http02(trailers))))
                    }
                    // we might get a trailers frame on next poll
                    // so loop and try again
                    Err(_frame) => {}
                },
                Some(Err(err)) => break Poll::Ready(Err(err)),
                None => break Poll::Ready(Ok(None)),
            }
        }
    }

    fn size_hint(&self) -> http_body_04::SizeHint {
        let size_hint = self.body.size_hint();
        let mut out = http_body_04::SizeHint::new();
        out.set_lower(size_hint.lower());
        if let Some(upper) = size_hint.upper() {
            out.set_upper(upper);
        }
        out
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }
}

fn http1_headermap_to_http02(h: http_1::HeaderMap) -> http_02::HeaderMap {
    let mut hm = http_02::HeaderMap::new();
    for (k, v) in h {
        hm.insert(
            http_02::HeaderName::from_str(k.unwrap().as_str()).unwrap(),
            http_02::HeaderValue::from_bytes(v.as_ref()).unwrap(),
        );
    }
    hm
}

fn http02_headermap_to_http1(h: http_02::HeaderMap) -> http_1::HeaderMap {
    let mut hm = http_1::HeaderMap::new();
    for (k, v) in h {
        hm.insert(
            http_1::HeaderName::from_str(k.unwrap().as_str()).unwrap(),
            http_1::HeaderValue::from_bytes(v.as_ref()).unwrap(),
        );
    }
    hm
}

pub fn http1_request_to_http02<B>(r: http_1::Request<B>) -> http_02::Request<B> {
    let (head, body) = r.into_parts();
    let mut build = http_02::request::Builder::new()
        .method(head.method.as_str())
        .uri(format!("{}", head.uri))
        .version(match head.version {
            http_1::version::Version::HTTP_09 => http_02::version::Version::HTTP_09,
            http_1::version::Version::HTTP_10 => http_02::version::Version::HTTP_10,
            http_1::version::Version::HTTP_11 => http_02::version::Version::HTTP_11,
            http_1::version::Version::HTTP_2 => http_02::version::Version::HTTP_2,
            http_1::version::Version::HTTP_3 => http_02::version::Version::HTTP_3,
            _ => panic!("unknown version"),
        });
    for (k, v) in head.headers {
        build = build.header(k.unwrap().as_str(), v.as_ref())
    }
    // for v in head.extension {
    //     build = build.extension(v)
    // }
    build.body(body).unwrap()
}

pub fn http1_response_to_http02<B>(r: http_1::Response<B>) -> http_02::Response<B> {
    let (head, body) = r.into_parts();
    let mut build = http_02::response::Builder::new()
        .status(head.status.as_u16())
        .version(match head.version {
            http_1::version::Version::HTTP_09 => http_02::version::Version::HTTP_09,
            http_1::version::Version::HTTP_10 => http_02::version::Version::HTTP_10,
            http_1::version::Version::HTTP_11 => http_02::version::Version::HTTP_11,
            http_1::version::Version::HTTP_2 => http_02::version::Version::HTTP_2,
            http_1::version::Version::HTTP_3 => http_02::version::Version::HTTP_3,
            _ => panic!("unknown version"),
        });
    for (k, v) in head.headers {
        build = build.header(k.unwrap().as_str(), v.as_ref())
    }
    // for v in head.extension {
    //     build = build.extension(v)
    // }
    build.body(body).unwrap()
}

pub fn http02_request_to_http1<B>(r: http_02::Request<B>) -> http_1::Request<B> {
    let (head, body) = r.into_parts();
    let mut build = http_1::request::Builder::new()
        .method(head.method.as_str())
        .uri(format!("{}", head.uri))
        .version(match head.version {
            http_02::version::Version::HTTP_09 => http_1::version::Version::HTTP_09,
            http_02::version::Version::HTTP_10 => http_1::version::Version::HTTP_10,
            http_02::version::Version::HTTP_11 => http_1::version::Version::HTTP_11,
            http_02::version::Version::HTTP_2 => http_1::version::Version::HTTP_2,
            http_02::version::Version::HTTP_3 => http_1::version::Version::HTTP_3,
            _ => panic!("unknown version"),
        });
    for (k, v) in head.headers {
        build = build.header(k.unwrap().as_str(), v.as_ref())
    }
    // for v in head.extension {
    //     build = build.extension(v)
    // }
    build.body(body).unwrap()
}

pub fn http02_response_to_http1<B>(r: http_02::Response<B>) -> http_1::Response<B> {
    let (head, body) = r.into_parts();
    let mut build = http_1::response::Builder::new()
        .status(head.status.as_u16())
        .version(match head.version {
            http_02::version::Version::HTTP_09 => http_1::version::Version::HTTP_09,
            http_02::version::Version::HTTP_10 => http_1::version::Version::HTTP_10,
            http_02::version::Version::HTTP_11 => http_1::version::Version::HTTP_11,
            http_02::version::Version::HTTP_2 => http_1::version::Version::HTTP_2,
            http_02::version::Version::HTTP_3 => http_1::version::Version::HTTP_3,
            _ => panic!("unknown version"),
        });
    for (k, v) in head.headers {
        build = build.header(k.unwrap().as_str(), v.as_ref())
    }
    // for v in head.extension {
    //     build = build.extension(v)
    // }
    build.body(body).unwrap()
}
