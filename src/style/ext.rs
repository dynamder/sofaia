pub trait StyleExt: Sized {
    type Kind;

    fn styled(self, kind: Self::Kind) -> Self;
}
