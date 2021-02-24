use std::error::Error;
pub trait Stream<TEvent>: Sized {
    fn stream<I>(&mut self, events: I) -> Result<usize, Box<dyn Error>>
    where
        I: IntoIterator<Item = TEvent>;
}

impl<S, TEvent> Stream<TEvent> for &mut S
where
    S: Stream<TEvent>,
{
    fn stream<I>(&mut self, events: I) -> Result<usize, Box<dyn Error>>
    where
        I: IntoIterator<Item = TEvent>,
    {
        (*self).stream(events)
    }
}

impl<TEvent> Stream<TEvent> for Vec<TEvent> {
    fn stream<I>(&mut self, events: I) -> Result<usize, Box<dyn Error>>
    where
        I: IntoIterator<Item = TEvent>,
    {
        let len_before = self.len();
        self.extend(events);
        let result = self.len() - len_before;
        Ok(result)
    }
}

pub struct StreamAdapter<TInner, F>(TInner, F);

impl<TInner, F> StreamAdapter<TInner, F> {
    pub fn new(original: TInner, f: F) -> Self {
        Self(original, f)
    }
}

impl<TInnerEvent, TEvent, TInner, F> Stream<TEvent> for StreamAdapter<TInner, F>
where
    TInner: Stream<TInnerEvent>,
    F: Fn(TEvent) -> TInnerEvent,
{
    fn stream<I>(&mut self, events: I) -> Result<usize, Box<dyn Error>>
    where
        I: IntoIterator<Item = TEvent>,
    {
        self.0.stream(events.into_iter().map(&self.1))
    }
}
