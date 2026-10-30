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
//! * [`Curve`] is **duration plus shape**, because easing is a modification of
//!   the curve and splitting them into two chained calls says otherwise.
//! * [`Track`] is the value: it records a target ([`Track::to`]), advances when
//!   told ([`Track::tick`]) and is read from a `view` ([`Track::value`]).
//! * [`animate`] is the whole functional interface: a value in, a value out.
//! * [`Motion`] names "does anything move", so [`frames`] can be the only place
//!   that decides whether the frame subscription is installed.
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
//! Repeat, delay and auto-reverse. Those are what `iced::Animation` is *for*,
//! and it can have them: it exposes an `Easing` enum rather than a shape
//! function, which is exactly the axis this module does not want. If a
//! blink-or-pulse needs a repeating animation, use `iced::Animation` directly.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use iced_core::time::{Duration, Instant};
use iced_futures::Subscription;

/// A duration and the shape of the motion across it.
///
/// The shape is a plain `fn(f32) -> f32` from unit progress to unit progress,
/// so `ease-in` is not a variant of an enum but a function — which is what it
/// is, and what makes `smootherstep` and a client's own spline the same kind of
/// thing.
#[derive(Debug, Clone, Copy)]
pub struct Curve {
    duration: Duration,
    shape_fn: fn(f32) -> f32,
}

impl Curve {
    /// A curve of the given duration, shaped by `shape`.
    ///
    /// `shape` takes progress in `0.0..=1.0` and returns the same range; it is
    /// expected to be monotone and to map `0` to `0` and `1` to `1`.
    pub const fn within(duration: Duration, shape: fn(f32) -> f32) -> Self {
        Self {
            duration,
            shape_fn: shape,
        }
    }

    /// Constant speed.
    pub const fn linear(duration: Duration) -> Self {
        Self::within(duration, shapes::linear)
    }

    /// Starts fast, settles into the target.
    pub const fn out(duration: Duration) -> Self {
        Self::within(duration, shapes::cubic_out)
    }

    /// Slow at both ends; the least mechanical of the three.
    pub const fn in_out(duration: Duration) -> Self {
        Self::within(duration, shapes::smootherstep)
    }

    /// How long the motion takes.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// The shape, from unit progress to unit progress.
    pub fn shape(&self, progress: f32) -> f32 {
        (self.shape_fn)(progress.clamp(0.0, 1.0))
    }
}

mod shapes {
    pub(super) fn linear(p: f32) -> f32 {
        p
    }

    pub(super) fn cubic_out(p: f32) -> f32 {
        1.0 - (1.0 - p).powi(3)
    }

    /// Perlin's smootherstep: zero first *and* second derivative at both ends.
    pub(super) fn smootherstep(p: f32) -> f32 {
        (6.0 * p.powi(5) - 15.0 * p.powi(4) + 10.0 * p.powi(3)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct Transition {
    from: f32,
    to: f32,
    started: Instant,
    curve: Curve,
}

impl Transition {
    fn at(&self, now: Instant) -> f32 {
        let total = self.curve.duration.as_secs_f32();

        if total <= 0.0 {
            return self.to;
        }

        let progress = (now.duration_since(self.started).as_secs_f32() / total).clamp(0.0, 1.0);

        self.from + (self.to - self.from) * self.curve.shape(progress)
    }

    fn done(&self, now: Instant) -> bool {
        now.duration_since(self.started) >= self.curve.duration
    }
}

#[derive(Debug)]
struct Inner {
    curve: Cell<Curve>,
    transition: RefCell<Transition>,
    /// A target that has been declared but not yet given a start time.
    pending: Cell<Option<f32>>,
    /// The last instant `tick` was given; what `value` is measured at.
    now: Cell<Instant>,
}

/// A value that moves towards a target.
///
/// Shared by cloning: the handle is cheap and `&Track` is enough to declare,
/// advance and read, because a `&self` `view` has to be able to read one.
#[derive(Debug, Clone)]
pub struct Track {
    inner: Rc<Inner>,
}

impl Track {
    /// A track that is already at `value` and not moving.
    ///
    /// The instant inside is an epoch, not a start: it is replaced by the first
    /// [`Track::tick`], and until then every projection returns `value`.
    pub fn new(value: f32) -> Self {
        Self {
            inner: Rc::new(Inner {
                curve: Cell::new(Curve::in_out(Duration::from_millis(200))),
                transition: RefCell::new(Transition {
                    from: value,
                    to: value,
                    started: Instant::now(),
                    curve: Curve::linear(Duration::ZERO),
                }),
                pending: Cell::new(None),
                now: Cell::new(Instant::now()),
            }),
        }
    }

    /// Sets the curve used by *later* transitions. Set it once, at construction.
    pub fn curve(self, curve: Curve) -> Self {
        self.inner.curve.set(curve);
        self
    }

    /// Declares where the value should end up.
    ///
    /// This does **not** start the motion: the start time comes from the next
    /// [`Track::tick`], so an animation always begins on a frame rather than on
    /// whatever instant the state changed at. Declaring the target that is
    /// already on its way is a no-op, which is what lets a `view`-adjacent
    /// caller repeat it every message without stalling the motion.
    pub fn to(&self, target: f32) {
        let inner = &*self.inner;

        let upcoming = inner
            .pending
            .get()
            .unwrap_or_else(|| inner.transition.borrow().to);

        if upcoming == target {
            return;
        }

        inner.pending.set(Some(target));
    }

    /// Advances the track to `now`, starting a declared target if there is one.
    ///
    /// Idempotent: calling it twice with the same instant changes nothing, and
    /// calling it from two owners (an app frame message and a widget's redraw
    /// request) is harmless. That is what makes the choice of clock source a
    /// choice rather than a commitment.
    pub fn tick(&self, now: Instant) {
        let inner = &*self.inner;

        inner.now.set(now);

        if let Some(target) = inner.pending.take() {
            let from = inner.transition.borrow().at(now);
            let curve = inner.curve.get();

            *inner.transition.borrow_mut() = Transition {
                from,
                to: target,
                started: now,
                curve,
            };
        }
    }

    /// The value as of the last ticked instant.
    ///
    /// This is what a `view` should draw: the frame that is on screen, not the
    /// one that would be there if time had passed while the message was being
    /// handled. Between ticks the value is exactly stable, so a rebuild caused
    /// by an unrelated message cannot make the picture jump ahead.
    pub fn value(&self) -> f32 {
        self.at(self.inner.now.get())
    }

    /// The value at an arbitrary instant, for a caller that already has one.
    pub fn at(&self, now: Instant) -> f32 {
        self.inner.transition.borrow().at(now)
    }

    /// Whether anything is left to do: a target waiting to start, or a motion
    /// that has not finished.
    ///
    /// A pending target counts, otherwise declaring one would never be ticked.
    pub fn is_moving(&self) -> bool {
        let inner = &*self.inner;

        inner.pending.get().is_some() || !inner.transition.borrow().done(inner.now.get())
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
pub fn frames<A, Message>(app: &A, f: fn(Instant) -> Message) -> Subscription<Message>
where
    A: Motion,
    Message: 'static,
{
    if app.is_moving() {
        iced_runtime::window::frames().map(f)
    } else {
        Subscription::none()
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
    fn a_new_track_is_not_moving() {
        let track = Track::new(3.0);

        assert_eq!(track.value(), 3.0);
        assert!(!track.is_moving());
    }

    #[test]
    fn declaring_a_target_does_not_start_it() {
        let track = Track::new(0.0);

        track.to(1.0);

        // Declared, so something is left to do…
        assert!(track.is_moving());
        // …but the clock has not moved, so the value has not either.
        assert_eq!(track.value(), 0.0);
    }

    #[test]
    fn the_first_tick_starts_the_motion() {
        let track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
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
        let track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
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
        let track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
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
        let track = Track::new(0.0).curve(Curve::linear(Duration::from_millis(200)));
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
    fn animate_is_only_a_read() {
        let track = Track::new(0.0);

        assert_eq!(animate(&track, |value| value * 2.0), 0.0);
    }

    #[test]
    fn a_zero_duration_curve_lands_immediately() {
        let track = Track::new(0.0).curve(Curve::linear(Duration::ZERO));

        track.to(1.0);
        track.tick(at(0));

        assert_eq!(track.value(), 1.0);
        assert!(!track.is_moving());
    }
}
