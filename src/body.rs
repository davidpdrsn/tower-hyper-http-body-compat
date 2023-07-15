use std::{
    pin::Pin,
    task::{Context, Poll},
};

use http::HeaderMap;
use http_body_1::Frame;
use pin_project_lite::pin_project;

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
            Poll::Ready(Ok(Some(trailers))) => Poll::Ready(Some(Ok(Frame::trailers(trailers)))),
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
    ) -> Poll<Result<Option<http::HeaderMap>, Self::Error>> {
        loop {
            let this = self.as_mut().project();

            if let Some(trailers) = this.trailers.take() {
                break Poll::Ready(Ok(Some(trailers)));
            }

            match ready!(this.body.poll_frame(cx)) {
                Some(Ok(frame)) => match frame.into_trailers() {
                    Ok(trailers) => break Poll::Ready(Ok(Some(trailers))),
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
