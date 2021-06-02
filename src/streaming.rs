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

pub fn pipe_streams<'a, S1, S2>(s1: &'a mut S1, s2: &'a mut S2) -> StreamPipe<'a, S1, S2> {
    StreamPipe { s1, s2 }
}

pub struct StreamPipe<'a, S1, S2> {
    s1: &'a mut S1,
    s2: &'a mut S2,
}

impl<'a, TEvent, S1, S2> Stream<TEvent> for StreamPipe<'a, S1, S2>
where
    TEvent: Clone,
    S1: Stream<TEvent>,
    S2: Stream<TEvent>,
{
    fn stream<I>(&mut self, events: I) -> Result<usize, Box<dyn Error>>
    where
        I: IntoIterator<Item = TEvent>,
    {
        // TODO: Make it more memory efficient
        let all: Vec<TEvent> = events.into_iter().collect();
        self.s1.stream(all.clone())?;
        self.s2.stream(all)
    }
}
