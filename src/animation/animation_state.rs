use bevy::{
    prelude::{Component, Entity, Handle},
    reflect::Reflect,
    time::Time,
};

use rose_data::AnimationEventFlags;

use crate::animation::ZmoAsset;

pub struct AnimationFrameEvent {
    pub entity: Entity,
    pub flags: AnimationEventFlags,
}

impl AnimationFrameEvent {
    pub fn new(entity: Entity, flags: AnimationEventFlags) -> Self {
        Self { entity, flags }
    }
}

#[derive(Reflect, Component)]
pub struct AnimationState {
    /// Currently playing animation asset
    motion: Handle<ZmoAsset>,

    /// Speed multiplier for the animation asset
    animation_speed: f32,

    /// How many times to repeat this animation before the animation completes,
    /// if None then this animation repeats forever
    max_loop_count: Option<usize>,

    /// The number of times this animation has been repeated
    current_loop_count: usize,

    /// The current interpolation weight for the ZmoAsset interpolation_interval
    interpolate_weight: f32,

    /// Whether this animation has completed or not
    completed: bool,

    /// The time this animation started.
    start_time: Option<f64>,

    /// The index of the current animation frame.
    current_frame_index: usize,

    /// The index of the next animation frame.
    next_frame_index: usize,

    /// The fraction of the current animation frame, this is effectively the blend weight
    /// between current and next frame.
    current_frame_fract: f32,

    /// This is used to track which frame events we have emitted so far
    last_absolute_event_frame: usize,

    /// Seconds to delay animation start by
    start_delay: Option<f32>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            motion: Default::default(),
            completed: false,
            max_loop_count: Some(1),
            animation_speed: 1.0,
            start_time: None,
            interpolate_weight: 0.0,
            current_loop_count: 0,
            current_frame_fract: 0.0,
            current_frame_index: 0,
            next_frame_index: 1,
            last_absolute_event_frame: 0,
            start_delay: None,
        }
    }
}

impl AnimationState {
    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self {
            motion,
            max_loop_count: Some(1),
            ..Default::default()
        }
    }

    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self {
            motion,
            max_loop_count: limit,
            ..Default::default()
        }
    }

    pub fn with_animation_speed(mut self, animation_speed: f32) -> Self {
        self.animation_speed = animation_speed;
        self
    }

    pub fn with_max_loop_count(mut self, max_loop_count: usize) -> Self {
        self.max_loop_count = Some(max_loop_count);
        self
    }

    pub fn set_animation_speed(&mut self, animation_speed: f32) {
        self.animation_speed = animation_speed;
    }

    pub fn set_completed(&mut self) {
        self.completed = true;
    }

    pub fn set_start_delay(&mut self, start_delay: f32) {
        if start_delay > 0.0 {
            self.start_delay = Some(start_delay);
        } else {
            self.start_delay = None;
        }
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn motion(&self) -> &Handle<ZmoAsset> {
        &self.motion
    }

    pub fn current_frame_index(&self) -> usize {
        self.current_frame_index
    }

    pub fn next_frame_index(&self) -> usize {
        self.next_frame_index
    }

    pub fn current_frame_fract(&self) -> f32 {
        self.current_frame_fract
    }

    pub fn current_loop_count(&self) -> usize {
        self.current_loop_count
    }

    pub fn interpolate_weight(&self) -> Option<f32> {
        if self.interpolate_weight < 1.0 {
            Some(self.interpolate_weight)
        } else {
            None
        }
    }

    /// Advance the animation, returns true if the animation has completed
    pub fn advance(&mut self, zmo_asset: &ZmoAsset, time: &Time) -> bool {
        if self.completed {
            return true;
        }

        if let Some(start_delay) = self.start_delay.as_mut() {
            *start_delay -= time.delta_seconds();
            if *start_delay > 0.0 {
                // Waiting until start time
                return false;
            } else {
                self.start_delay = None;
            }
        }

        let current_time = time.elapsed_seconds_f64();
        let start_time = if let Some(start_time) = self.start_time {
            start_time
        } else {
            self.start_time = Some(current_time);
            current_time
        };

        if self.interpolate_weight < 1.0 {
            self.interpolate_weight += time.delta_seconds() / zmo_asset.interpolation_interval;
        }

        let animation_frame_number =
            (current_time - start_time) * (zmo_asset.fps as f64) * self.animation_speed as f64;

        self.current_loop_count = animation_frame_number as usize / zmo_asset.num_frames;
        self.completed = self.current_loop_count >= self.max_loop_count.unwrap_or(usize::MAX);

        if self.completed {
            self.current_frame_fract = 0.0;
            self.current_frame_index = zmo_asset.num_frames - 1;
            self.next_frame_index = self.current_frame_index;
            self.current_loop_count = self.max_loop_count.unwrap() - 1;
        } else {
            self.current_frame_fract = animation_frame_number.fract() as f32;
            self.current_frame_index = animation_frame_number as usize % zmo_asset.num_frames;
            self.next_frame_index = if self.current_frame_index + 1 == zmo_asset.num_frames
                && self.current_loop_count + 1 >= self.max_loop_count.unwrap_or(usize::MAX)
            {
                // The last frame of last loop should not blend to the first frame
                self.current_frame_index
            } else {
                (self.current_frame_index + 1) % zmo_asset.num_frames
            };
        }

        self.completed
    }

    pub fn iter_animation_events(
        &mut self,
        zmo_asset: &ZmoAsset,
        mut event_handler: impl FnMut(u16),
    ) {
        let num_frames = zmo_asset.num_frames;
        let current_event_frame = self.current_frame_index + self.current_loop_count * num_frames;

        while self.last_absolute_event_frame <= current_event_frame {
            if let Some(event_id) =
                zmo_asset.get_frame_event(self.last_absolute_event_frame % num_frames)
            {
                event_handler(event_id.get());
            }

            self.last_absolute_event_frame += 1;
        }
    }
}
