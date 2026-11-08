//! Minimaler Ersatz für `Promise.all`: wartet auf mehrere Futures
//! gleichzeitig und liefert die Ergebnisse in Eingabereihenfolge.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct JoinAll<F: Future> {
    futs: Vec<Option<Pin<Box<F>>>>,
    out: Vec<Option<F::Output>>,
}

impl<F: Future> Unpin for JoinAll<F> {}

impl<F: Future> Future for JoinAll<F> {
    type Output = Vec<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let mut pending = false;
        for (slot, out) in this.futs.iter_mut().zip(this.out.iter_mut()) {
            if let Some(f) = slot {
                match f.as_mut().poll(cx) {
                    Poll::Ready(v) => {
                        *out = Some(v);
                        *slot = None;
                    }
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(this.out.iter_mut().map(|o| o.take().expect("Ergebnis fehlt")).collect())
        }
    }
}

pub fn join_all<F: Future>(futs: impl IntoIterator<Item = F>) -> JoinAll<F> {
    let futs: Vec<_> = futs.into_iter().map(|f| Some(Box::pin(f))).collect();
    let out = futs.iter().map(|_| None).collect();
    JoinAll { futs, out }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::task::{Wake, Waker};

    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }

    #[test]
    fn keeps_order() {
        let waker = Waker::from(Arc::new(Noop));
        let mut cx = Context::from_waker(&waker);
        let mut j = join_all((1..=3).map(|i| async move { i * 2 }));
        match Pin::new(&mut j).poll(&mut cx) {
            Poll::Ready(v) => assert_eq!(v, vec![2, 4, 6]),
            Poll::Pending => panic!("sollte fertig sein"),
        }
        let mut empty = join_all(Vec::<std::future::Ready<i32>>::new());
        assert!(matches!(Pin::new(&mut empty).poll(&mut cx), Poll::Ready(v) if v.is_empty()));
    }
}
