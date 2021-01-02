pub trait InContext: Sized {
    fn in_context<TCtx>(self, context: TCtx) -> Contextual<Self, TCtx>;
}

impl<T: Sized> InContext for T {
    fn in_context<TCtx>(self, context: TCtx) -> Contextual<Self, TCtx> {
        Contextual {
            subject: self,
            context,
        }
    }
}

pub struct Contextual<T, TCtx> {
    pub subject: T,
    pub context: TCtx,
}
