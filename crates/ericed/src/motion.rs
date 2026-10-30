//! A value that moves towards a target, and the clock that moves it.
//!
//! iced has the pieces — `iced::Animation` interpolates, `RedrawRequested` is a
//! frame, `Subscription` can turn one into a message — but nothing joins them,
//! and the join is where every application gets it slightly wrong: a clock read
//! in three places, a subscription that is never turned off, an animation that
//! starts from a stale instant.
//!
//! This module is that join, and it is deliberately small:
//!
//! * [`Curve`] is **duration plus an iced [`Easing`]**, because easing is a
//!   modification of the curve and splitting them into two chained calls says
//!   otherwise. `Easing::Custom` takes a `fn(f32) -> f32`, so a hand-written
//!   shape is not a second mechanism.
//! * [`Track`] is the value: it records a target ([`Track::to`]), advances when
//!   told ([`Track::tick`]) and is read from a `view` ([`Track::value`]). The
//!   keyframes and the projection are iced's `Animation<f32>`; the contract
//!   around them — declared now, started on a frame, stable between frames — is
//!   the part iced does not have.
//! * [`animate`] is the whole functional interface: a value in, a value out.
//! * [`Motion`] names "does anything move", so [`frames`] can be the only place
//!   that decides whether the frame subscription is installed.
//! * [`transform`] is what a value is *applied* with — a draw-time
//!   [`Placement`], not a layout change. [`translate`] and [`scale`] are the two
//!   constructors, and between them they cover what iced's renderer can
//!   honestly draw.
//!
//! # A moving value must not move the layout
//!
//! The obvious way to slide something with iced is to put it at an x coordinate:
//! `pin(content).x(offset)`. It is also wrong, and quietly so: `pin` moves the
//! *layout node*, and the child is laid out with `limits.max() - position`
//! available. A pane sliding in from the right is therefore re-laid-out at a
//! narrower width on every frame — text re-wraps, borders shorten, and the
//! "animation" is really a resize.
//!
//! [`translate`] moves the drawing instead. The child keeps its layout, its
//! width and its shape, and the offset only reaches the pixels. That is the
//! difference between an animation and a per-frame relayout, and it is not
//! something a `Track` can express on its own: a value needs somewhere to go.
//!
//! # The clock has two possible owners, and the code is the same
//!
//! `Track` never reads a clock. It is advanced by [`Track::tick`], which is
//! idempotent, so either owner works without changing a line of drawing code:
//!
//! ```text
//! App::update(Msg::Frame(now)) ──► track.tick(now) ──► view: track.value()
//! widget::update(RedrawRequested(now)) ──► same tick ──► draw: same value
//! ```
//!
//! The first is pure — time arrives as a message, so the animation is a
//! function of the message stream and can be replayed and snapshot-tested. It
//! costs a full `view` + `layout` per frame, which for a small application is
//! nothing and for a desktop shell is everything.
//!
//! The second is iced's own trick (`text_input`'s caret, every `request_redraw`
//! in a widget): no messages, no rebuild, only a repaint. It cannot be replayed,
//! because the instant comes from the widget.
//!
//! Pick per animation, not per application.
//!
//! # What is not here
//!
//! Repeat, delay and auto-reverse: a [`Curve`] is one transition, and those are
//! what `iced::Animation` is for. But the division is not only cosmetic — it is
//! what iced's animation can *hold*. `Animation<T>` interpolates a state that
//! maps onto a single `f32` keyframe: `FloatRepresentable` is implemented for
//! `bool` and `f32` and for nothing else, and neither it nor `Interpolable` can
//! be implemented for `Color` or `Vector` outside iced, because the trait and
//! the type would both be foreign. So a value that is not one number cannot be
//! an `Animation` state, and a `Color` cannot be its projection.
//!
//! [`Track`] is exactly that gap: one number moving, with the projection —
//! a `Vector`, a `Color`, a whole `Tokens` — left to the caller. Everything
//! below the gap is iced's: [`Curve::easing`] is an `Easing`, `Track`'s
//! keyframes are an `Animation<f32>`, and the shapes are the thirty iced
//! already ships plus `Easing::Custom`.

use iced_core::animation::{Animation, Easing};
use iced_core::layout;
use iced_core::mouse;
use iced_core::overlay;
use iced_core::renderer;
use iced_core::time::{Duration, Instant};
use iced_core::widget::tree::{self, Tree};
use iced_core::widget::{self, Widget};
use iced_core::{
    Clipboard, Element, Event, Layout, Length, Point, Rectangle, Shell, Size, Transformation,
    Vector,
};
use iced_futures::Subscription;

/// A duration and the shape of the motion across it.
///
/// The shape is iced's [`Easing`]: thirty named curves, plus [`Easing::Custom`],
/// which takes a `fn(f32) -> f32`. A hand-written spline is therefore not a
/// second mechanism — [`Curve::within`] *is* `Easing::Custom` spelled as a
/// function, and everything iced names is reachable through [`Curve::eased`].
///
/// A shape may leave the unit interval. `EaseInBack`, `EaseOutElastic` and
/// friends overshoot on purpose — that is the whole point of a pop-in — so
/// [`Track::value`] on such a curve overshoots its target and lands outside the
/// range between the two ends. Positioning something usually wants exactly
/// that; mixing two `Color`s should go through `iced::theme::palette::mix`,
/// which clamps.
#[derive(Debug, Clone, Copy)]
pub struct Curve {
    duration: Duration,
    easing: Easing,
}

impl Curve {
    /// A curve of the given duration, shaped by `easing`.
    ///
    /// This is the whole of iced's easing catalogue, which is the reason this
    /// constructor exists at all: `Curve::out` is a convenience, not a limit.
    pub const fn eased(duration: Duration, easing: Easing) -> Self {
        Self { duration, easing }
    }

    /// A curve of the given duration, shaped by `shape`.
    ///
    /// `shape` takes progress in `0.0..=1.0` and returns progress. It is
    /// [`Easing::Custom`] spelled as a function: it is expected to map `0` to
    /// `0` and `1` to `1`, and nothing here checks that it does.
    pub const fn within(duration: Duration, shape: fn(f32) -> f32) -> Self {
        Self::eased(duration, Easing::Custom(shape))
    }

    /// Constant speed.
    pub const fn linear(duration: Duration) -> Self {
        Self::eased(duration, Easing::Linear)
    }

    /// Starts fast, settles into the target.
    pub const fn out(duration: Duration) -> Self {
        Self::eased(duration, Easing::EaseOutCubic)
    }

    /// Slow at both ends; the least mechanical of the three.
    pub const fn in_out(duration: Duration) -> Self {
        Self::within(duration, shapes::smootherstep)
    }

    /// How long the motion takes.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// The shape, as iced names it.
    ///
    /// This is what [`Track`] hands to `Animation`, so the easing is iced's
    /// data all the way down; `Curve` only adds "and for how long".
    pub fn easing(&self) -> Easing {
        self.easing
    }

    /// The shape, from unit progress to unit progress.
    pub fn shape(&self, progress: f32) -> f32 {
        self.easing.value(progress.clamp(0.0, 1.0))
    }
}

mod shapes {
    /// Perlin's smootherstep: zero first *and* second derivative at both ends.
    ///
    /// iced has no name for this one — `EaseInOutQuint` is close, but its second
    /// derivative at the ends is not zero — so it stays a `Custom` shape rather
    /// than being approximated by a named easing.
    pub(super) fn smootherstep(p: f32) -> f32 {
        (6.0 * p.powi(5) - 15.0 * p.powi(4) + 10.0 * p.powi(3)).clamp(0.0, 1.0)
    }
}

/// A value that moves towards a target.
///
/// A plain value: `update` mutates it, `view` reads it. No interior mutability
/// and no shared handle, because nothing but the message carrying the instant
/// should be able to move an animation — the same shape iced's own stateful
/// widgets have (`&mut` in `update`, `&self` in `view`).
///
/// [`Clone`] copies the state, not the handle: a cloned track is a second
/// animation that happens to start from the same value. That is what makes a
/// track per row, or per kind, cheap to write down.
#[derive(Debug, Clone)]
pub struct Track {
    curve: Curve,
    /// iced's interpolator: two keyframes, the instant the transition started at,
    /// and the projection of a value at an instant.
    ///
    /// What it does *not* have is the contract around it — a target declared now
    /// and started on a frame, a value that does not move between frames — and
    /// that is the rest of this struct.
    animation: Animation<f32>,
    /// A target that has been declared but not yet given a start time.
    pending: Option<f32>,
    /// The last instant `tick` was given; what `value` is measured at.
    now: Instant,
}

impl Track {
    /// A track that is already at `value` and not moving.
    ///
    /// The instant inside is an epoch, not a start: it is replaced by the first
    /// [`Track::tick`], and until then every projection returns `value`.
    pub fn new(value: f32) -> Self {
        Self {
            curve: Curve::in_out(Duration::from_millis(200)),
            animation: Animation::new(value),
            pending: None,
            now: Instant::now(),
        }
    }

    /// Sets the curve used by *later* transitions. Set it once, at construction.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Declares where the value should end up.
    ///
    /// This does **not** start the motion: the start time comes from the next
    /// [`Track::tick`], so an animation always begins on a frame rather than on
    /// whatever instant the state changed at. Declaring the target that is
    /// already on its way is a no-op, so an `update` can declare it on every
    /// message — every frame of a drag, say — without stalling the motion.
    pub fn to(&mut self, target: f32) {
        let upcoming = self.pending.unwrap_or_else(|| self.animation.value());

        if upcoming == target {
            return;
        }

        self.pending = Some(target);
    }

    /// Advances the track to `now`, starting a declared target if there is one.
    ///
    /// Idempotent: calling it twice with the same instant changes nothing. That
    /// is what lets either clock owner use the same code — an application frame
    /// message calling this in `update`, or a widget calling it from
    /// `RedrawRequested` and reading the value later in `draw`.
    pub fn tick(&mut self, now: Instant) {
        self.now = now;

        let Some(target) = self.pending.take() else {
            return;
        };

        // iced's builders take `self` by value, so the settings ride along on a
        // copy of the interpolation. One clone per *transition*, never per frame.
        self.animation = self
            .animation
            .clone()
            .duration(self.curve.duration())
            .easing(self.curve.easing());

        // `go_mut` starts from the value that is on screen — iced computes the
        // interrupted keyframe itself — so a target declared mid-flight does not
        // jump back to where the previous motion began.
        self.animation.go_mut(target, now);
    }

    /// The value as of the last ticked instant.
    ///
    /// This is what a `view` should draw: the frame that is on screen, not the
    /// one that would be there if time had passed while the message was being
    /// handled. Between ticks the value is exactly stable, so a rebuild caused
    /// by an unrelated message cannot make the picture jump ahead.
    ///
    /// An overshooting curve (`EaseOutBack` and friends) can put this outside
    /// the range between the previous value and the target; that is the curve's
    /// doing, not a bug.
    pub fn value(&self) -> f32 {
        self.at(self.now)
    }

    /// The value at an arbitrary instant, for a caller that already has one.
    pub fn at(&self, now: Instant) -> f32 {
        self.animation.interpolate_with(|value| value, now)
    }

    /// Whether anything is left to do: a target waiting to start, or a motion
    /// that has not finished.
    ///
    /// A pending target counts, otherwise declaring one would never be ticked.
    pub fn is_moving(&self) -> bool {
        self.pending.is_some() || self.animation.is_animating(self.now)
    }
}

/// Applies a track's current value to whatever the caller wants to build.
///
/// `f` is a plain `fn(f32) -> B`, which is the whole of the "functional" part:
/// a style, an element, a vector, a length. What differs between call sites is
/// *when* it runs, and that is the clock's business, not this function's.
pub fn animate<B>(track: &Track, f: impl FnOnce(f32) -> B) -> B {
    f(track.value())
}

/// "Is anything in here moving?"
///
/// Implemented by the application so that [`frames`] can ask one question
/// instead of the application remembering to ask at all.
pub trait Motion {
    fn is_moving(&self) -> bool;
}

/// The frame subscription, installed only while something moves.
///
/// iced's `window::frames()` is a hook on `RedrawRequested`: it reports frames
/// that already happened. If it is subscribed unconditionally the application
/// redraws forever; if it is subscribed conditionally, the condition has to be
/// somewhere. This is that somewhere.
///
/// `f` has to be a function item or a **non-capturing** closure, because
/// `Subscription::map` requires a zero-sized mapper (iced checks this at compile
/// time). A mapper that needs state should be `Subscription::with` instead —
/// though a frame mapper rarely does, since the state it would want is the
/// instant it is being handed.
///
/// ```no_run
/// # use ericed::motion::{self, Motion};
/// # use iced_core::time::Instant;
/// # use iced_futures::Subscription;
/// # #[derive(Clone)] enum Message { Frame(Instant) }
/// # struct App;
/// # impl Motion for App { fn is_moving(&self) -> bool { false } }
/// # let app = App;
/// let subscription: Subscription<Message> = motion::frames(&app, Message::Frame);
/// ```
pub fn frames<A, F, Message>(app: &A, f: F) -> Subscription<Message>
where
    A: Motion,
    F: Fn(Instant) -> Message + Clone + iced_futures::MaybeSend + 'static,
    Message: 'static,
{
    if app.is_moving() {
        iced_runtime::window::frames().map(f)
    } else {
        Subscription::none()
    }
}

/// Where a [`Transform`] puts its content.
///
/// The set is closed on purpose. iced exposes translations and *uniform* scales
/// (`iced_core::Transformation`) and nothing else: no rotation, no skew, no
/// non-uniform scale, because `scale_factor()` is a single number and both
/// backends use it to size glyphs, borders and shadows. An enum makes that
/// limit visible in the type; a raw `Transformation` would let a caller write a
/// matrix the renderer cannot honestly draw.
///
/// Two kinds, because a transformation can be known at two different times:
/// an absolute one before layout, and a scaling whose pivot is a property of
/// the content's own box, which only exists after layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// The same transformation wherever the content lands.
    Absolute(Transformation),
    /// A scaling about a point of the content's own layout.
    Scale { factor: f32, about: Anchor },
}

impl Placement {
    /// The matrix this placement means for a content laid out at `bounds`.
    fn resolve(&self, bounds: Rectangle) -> Transformation {
        match *self {
            Self::Absolute(transformation) => transformation,
            Self::Scale { factor, about } => {
                let pivot = about.of(bounds);

                Transformation::translate(pivot.x, pivot.y)
                    * Transformation::scale(factor)
                    * Transformation::translate(-pivot.x, -pivot.y)
            }
        }
    }
}

/// A point of a content's own box to scale about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl Anchor {
    /// Where this anchor sits in `bounds`.
    pub fn of(self, bounds: Rectangle) -> Point {
        match self {
            Self::TopLeft => bounds.position(),
            Self::TopRight => Point::new(bounds.x + bounds.width, bounds.y),
            Self::BottomLeft => Point::new(bounds.x, bounds.y + bounds.height),
            Self::BottomRight => Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
            Self::Center => bounds.center(),
        }
    }
}

/// A widget drawn through a [`Transformation`], without being laid out through
/// one.
///
/// Build one with [`translate`], [`scale`] or [`transform`].
pub struct Transform<'a, Message, Theme = iced_widget::Theme, Renderer = iced_widget::Renderer>
where
    Renderer: iced_core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    placement: Placement,
    clip: bool,
}

/// Draws `content` shifted by `offset`, leaving the layout exactly as it was.
///
/// This is how a moving value reaches the screen: `offset` is normally
/// `track.value()` scaled by whatever the animation measures in. Because the
/// offset is applied while drawing, the child keeps its width and is not
/// re-laid-out — which is the one thing a layout offset cannot promise.
///
/// The catch is that the offset is invisible to layout: the space the content
/// *would* occupy is still its unshifted box, so a caller that wants the shift
/// clipped (a pane sliding over its neighbours) asks for [`Transform::clip`].
/// Hit testing is adjusted, so the content stays clickable while it moves.
pub fn translate<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    offset: Vector,
) -> Transform<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    transform(
        content,
        Placement::Absolute(Transformation::translate(offset.x, offset.y)),
    )
}

/// Draws `content` scaled by `factor` about its own centre, leaving the layout
/// exactly as it was.
///
/// `factor` is uniform, because iced's `Transformation` has one scale factor —
/// see [`Placement`]. A factor below `1.0` works: unlike `iced_widget::Float`,
/// which only floats *out* of its bounds, this scales the drawing either way.
pub fn scale<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    factor: f32,
) -> Transform<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    transform(
        content,
        Placement::Scale {
            factor,
            about: Anchor::Center,
        },
    )
}

/// Draws `content` through `placement`, leaving the layout exactly as it was.
///
/// # What this widget owes its child
///
/// Transforming a subtree is five obligations, and forgetting any one of them
/// is a bug that only shows up when the content moves:
///
/// | what | how |
/// | --- | --- |
/// | layout | forwarded untouched — the whole point |
/// | painting | `with_transformation` around the child's draw |
/// | clipping | the child is handed `viewport * inverse` |
/// | hit testing | the child is handed `cursor * inverse` |
/// | overlays | `translation + transformation.translation()` |
///
/// The last one is a limitation, not a trick: iced's `overlay::Element` carries
/// an additive `translation` and no matrix, so a popup inside a *scaled*
/// subtree can only be moved, not scaled. Translation is exact; scaling is an
/// approximation, and this paragraph is where that is admitted.
///
/// # Not the same thing as `iced_widget::Float`
///
/// `Float` uses the same renderer primitive but leaves the tree: when it floats
/// it draws nothing in place and re-draws itself through `overlay`, which loses
/// the ancestors' clipping and puts it above everything. That is what "floating
/// over the layout" means. This widget stays a widget — it is clipped by its
/// parents, ordered by the tree, and hit-testable in place — which is what
/// "moving inside the layout" means. If the intent is a translucent panel above
/// the application, use `Float`; if it is a pane sliding in from the side, use
/// this.
pub fn transform<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    placement: Placement,
) -> Transform<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    Transform {
        content: content.into(),
        placement,
        clip: false,
    }
}

impl<'a, Message, Theme, Renderer> Transform<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    /// Clips the drawing to the box the content was laid out in.
    ///
    /// Off by default, because a transformation is invisible to layout and a
    /// caller that wants the overflow gone has to say so. On, it is the same
    /// `container(..).clip(true)` region — the *untransformed* box — which is
    /// what a slide wants (a pane must not paint over its neighbours) and what
    /// a growth usually does not (a card getting bigger would be trimmed at its
    /// original edges).
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }
}

/// The window-space cursor, moved into the child's own space.
///
/// `Cursor * Transformation` applies the matrix; it does not invert, so the
/// inverse is the caller's job. This is the same line iced's own `Float` writes
/// (`cursor * inverse`), and it is the reason the child stays clickable while
/// it moves.
fn into_content(cursor: mouse::Cursor, transformation: Transformation) -> mouse::Cursor {
    cursor * transformation.inverse()
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Transform<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    // The whole point: the layout is the child's, untouched by the offset.
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let transformation = self.placement.resolve(layout.bounds());

        self.content.as_widget_mut().update(
            tree,
            event,
            layout,
            into_content(cursor, transformation),
            renderer,
            clipboard,
            shell,
            &(*viewport * transformation.inverse()),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let transformation = self.placement.resolve(layout.bounds());

        self.content.as_widget().mouse_interaction(
            tree,
            layout,
            into_content(cursor, transformation),
            &(*viewport * transformation.inverse()),
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let transformation = self.placement.resolve(bounds);
        let cursor = into_content(cursor, transformation);
        let viewport = &(*viewport * transformation.inverse());

        let paint = |renderer: &mut Renderer| {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        };

        // Three ways to get the same pixels: clipped, moved, or neither. The
        // identity case is not an optimisation for its own sake — most frames
        // of most animations are the settled ones, and a renderer group per
        // frame is a layer per frame.
        if self.clip {
            renderer.with_layer(bounds, paint);
        } else if transformation == Transformation::IDENTITY {
            paint(renderer);
        } else {
            renderer.with_transformation(transformation, paint);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let transformation = self.placement.resolve(layout.bounds());

        // Overlays carry a translation, not a matrix: the popup follows a moving
        // content exactly, and a scaling one only as far as its translation
        // component. `Placement::Scale` says the same thing in the docs.
        self.content.as_widget_mut().overlay(
            tree,
            layout,
            renderer,
            &(*viewport * transformation.inverse()),
            translation + transformation.translation(),
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Transform<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a,
{
    fn from(transformed: Transform<'a, Message, Theme, Renderer>) -> Self {
        Self::new(transformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(millis: u64) -> Instant {
        Instant::now() + Duration::from_millis(millis)
    }

    #[test]
    fn a_curve_maps_the_unit_interval_onto_itself() {
        for curve in [
            Curve::linear(Duration::from_millis(100)),
            Curve::out(Duration::from_millis(100)),
            Curve::in_out(Duration::from_millis(100)),
        ] {
            assert_eq!(curve.shape(0.0), 0.0);
            assert_eq!(curve.shape(1.0), 1.0);
            assert!(curve.shape(0.5) > 0.0 && curve.shape(0.5) < 1.0);
            // Out of range progress is clamped rather than extrapolated.
            assert_eq!(curve.shape(-1.0), 0.0);
            assert_eq!(curve.shape(2.0), 1.0);
        }
    }

    #[test]
    fn the_named_curves_are_iced_easings() {
        let hundred = Duration::from_millis(100);

        // The convenience constructors are names for `Easing` variants, not a
        // second catalogue: an easing iced names is reachable without ericed
        // growing a variant for it.
        assert_eq!(Curve::linear(hundred).easing(), Easing::Linear);
        assert_eq!(Curve::out(hundred).easing(), Easing::EaseOutCubic);

        // `within` is `Easing::Custom` spelled as a function.
        assert!(matches!(
            Curve::within(hundred, shapes::smootherstep).easing(),
            Easing::Custom(_)
        ));
        assert!(matches!(Curve::in_out(hundred).easing(), Easing::Custom(_)));

        assert_eq!(Curve::eased(hundred, Easing::EaseOutBack).shape(0.0), 0.0);
    }

    #[test]
    fn an_overshooting_easing_goes_past_the_target() {
        let mut track = Track::new(0.0).curve(Curve::eased(
            Duration::from_millis(200),
            Easing::EaseOutBack,
        ));
        let start = at(0);

        track.to(1.0);
        track.tick(start);
        track.tick(start + Duration::from_millis(100));

        // `EaseOutBack` overshoots on purpose, and it is the *curve* that
        // decides that — not a `Track` that happens to be wrong.
        assert!(track.value() > 1.0, "got {}", track.value());

        // It still lands exactly on the target.
        track.tick(start + Duration::from_millis(200));
        assert_eq!(track.value(), 1.0);
        assert!(!track.is_moving());
    }

    #[test]
    fn a_new_track_is_not_moving() {
        let track = Track::new(3.0);

        assert_eq!(track.value(), 3.0);
        assert!(!track.is_moving());
    }

    #[test]
    fn declaring_a_target_does_not_start_it() {
        let mut track = Track::new(0.0);

        track.to(1.0);

        // Declared, so something is left to do…
        assert!(track.is_moving());
        // …but the clock has not moved, so the value has not either.
        assert_eq!(track.value(), 0.0);
    }

    #[test]
    fn the_first_tick_starts_the_motion() {
        let mut track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
        let start = at(0);

        track.to(1.0);
        track.tick(start);

        assert_eq!(track.value(), 0.0);
        assert!(track.is_moving());

        track.tick(start + Duration::from_millis(100));
        assert!((track.value() - 0.5).abs() < 1e-4);

        track.tick(start + Duration::from_millis(200));
        assert_eq!(track.value(), 1.0);
        assert!(!track.is_moving());
    }

    #[test]
    fn a_value_between_ticks_is_stable() {
        let mut track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
        let start = at(0);

        track.to(1.0);
        track.tick(start);
        track.tick(start + Duration::from_millis(100));

        let sampled = track.value();

        // Reading at a later instant is a different question, and asking it
        // must not change what a `view` sees.
        assert!(track.at(start + Duration::from_millis(300)) == 1.0);
        assert_eq!(track.value(), sampled);
    }

    #[test]
    fn a_second_target_continues_from_where_the_value_is() {
        let mut track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
        let start = at(0);

        track.to(1.0);
        track.tick(start);
        track.tick(start + Duration::from_millis(100));

        let interrupted = track.value();
        assert!((interrupted - 0.5).abs() < 1e-4);

        track.to(0.0);
        track.tick(start + Duration::from_millis(100));

        // The new motion starts at the value that was on screen, not at 1.0.
        assert!((track.value() - interrupted).abs() < 1e-4);

        track.tick(start + Duration::from_millis(300));
        assert_eq!(track.value(), 0.0);
    }

    #[test]
    fn repeating_the_same_target_does_not_stall_the_motion() {
        let mut track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
        let start = at(0);

        track.to(1.0);

        // A caller that declares the target on every message must not reset the
        // motion on every message.
        for step in 0..=4 {
            track.to(1.0);
            track.tick(start + Duration::from_millis(step * 50));
        }

        assert!((track.value() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn translating_does_not_change_the_size() {
        use iced_core::widget::Widget as _;

        let content: Element<'_, (), iced_widget::Theme, iced_widget::Renderer> =
            iced_widget::container(iced_widget::text("x")).into();

        let laid_out = content.as_widget().size();

        // A layout-based offset would hand the child a narrower `Limits` and
        // the size would change with it. A drawing offset must not.
        let moved = translate(content, Vector::new(120.0, 0.0));

        assert_eq!(moved.size(), laid_out);
    }

    #[test]
    fn scaling_does_not_change_the_size() {
        use iced_core::widget::Widget as _;

        let content: Element<'_, (), iced_widget::Theme, iced_widget::Renderer> =
            iced_widget::container(iced_widget::text("x")).into();

        let laid_out = content.as_widget().size();

        // The same promise as `translate`, and the reason `Float` cannot stand
        // in for it: a scaled widget is still laid out at its own size.
        let grown = scale(content, 1.5);

        assert_eq!(grown.size(), laid_out);
    }

    #[test]
    fn an_anchor_is_a_point_of_the_contents_own_box() {
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));

        assert_eq!(Anchor::TopLeft.of(bounds), Point::new(10.0, 20.0));
        assert_eq!(Anchor::TopRight.of(bounds), Point::new(110.0, 20.0));
        assert_eq!(Anchor::BottomLeft.of(bounds), Point::new(10.0, 70.0));
        assert_eq!(Anchor::BottomRight.of(bounds), Point::new(110.0, 70.0));
        assert_eq!(Anchor::Center.of(bounds), Point::new(60.0, 45.0));
    }

    #[test]
    fn scaling_keeps_its_pivot_fixed_and_moves_the_rest() {
        let bounds = Rectangle::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0));
        let doubling = Placement::Scale {
            factor: 2.0,
            about: Anchor::Center,
        };

        let transformation = doubling.resolve(bounds);

        // The pivot does not move…
        assert_eq!(bounds.center() * transformation, bounds.center());
        // …and a corner moves away from it, by the factor.
        assert_eq!(
            Point::new(0.0, 0.0) * transformation,
            Point::new(-50.0, -50.0)
        );
    }

    #[test]
    fn the_cursor_maps_back_into_the_untransformed_content() {
        // Obligations three and four of the contract: the child is asked about
        // its *own* coordinates, so a click on the scaled picture has to land on
        // the point that produced that pixel.
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
        let transformation = Placement::Scale {
            factor: 2.0,
            about: Anchor::Center,
        }
        .resolve(bounds);

        let content_point = Point::new(30.0, 25.0);
        let on_screen = content_point * transformation;

        let cursor = into_content(mouse::Cursor::Available(on_screen), transformation);

        assert_eq!(cursor.position(), Some(content_point));
    }

    #[test]
    fn clipping_is_off_until_it_is_asked_for() {
        fn content<'a>() -> Element<'a, (), iced_widget::Theme, iced_widget::Renderer> {
            iced_widget::text("x").into()
        }

        assert!(!translate(content(), Vector::ZERO).clip);
        assert!(!scale(content(), 1.2).clip);
        assert!(scale(content(), 1.2).clip(true).clip);
    }

    #[test]
    fn animate_is_only_a_read() {
        let track = Track::new(0.0);

        assert_eq!(animate(&track, |value| value * 2.0), 0.0);
    }

    #[test]
    fn a_zero_duration_curve_lands_immediately() {
        let mut track = Track::new(0.0).curve(Curve::linear(Duration::ZERO));

        track.to(1.0);
        track.tick(at(0));

        assert_eq!(track.value(), 1.0);
        assert!(!track.is_moving());
    }
}
