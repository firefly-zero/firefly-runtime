const BUF_SIZE: usize = 5;
const MAX_DRIFT: u32 = BUF_SIZE as u32 / 2;

/// Circular buffer designed to keep a short history and a short look-ahead
/// for netowrk device state updates. The goal is to be able to reply recent frames
/// as well as not to loose frames received earlier than expected.
#[derive(Debug)]
pub(crate) struct RingBuf<T: Copy> {
    /// The current frame.
    frame: u32,
    /// The circular buffer of values for each frame.
    data: [Option<(u32, T)>; BUF_SIZE],
}

impl<T: Copy> RingBuf<T> {
    const INIT: Option<(u32, T)> = None;

    pub fn new() -> Self {
        Self {
            frame: 0,
            data: [Self::INIT; BUF_SIZE],
        }
    }

    /// Increment the current frame number.
    pub fn advance(&mut self) {
        self.frame += 1
    }

    /// Set the value for the given frame.
    ///
    /// Frames inserted too far ahead or behind the current frame will be discarded.
    pub fn insert(&mut self, frame: u32, val: T) {
        // Max drift ensures that too old or too ahead frame doesn't override
        // the frame that is closer to what we currently need.
        if self.frame.abs_diff(frame) > MAX_DRIFT {
            return;
        }
        let index = frame as usize % BUF_SIZE;
        self.data[index] = Some((frame, val));
    }

    /// Get the value for the current frame.
    pub fn get_current(&self) -> Option<T> {
        self.get(self.frame)
    }

    /// Get the value for the given frame.
    ///
    /// The given frame must be not too far ahead or behind the current frame
    /// and must have been inserted earlier into the buffer.
    /// If any of this isn't true, None is returned.
    pub fn get(&self, frame: u32) -> Option<T> {
        if self.frame.abs_diff(frame) > MAX_DRIFT {
            return None;
        }
        let index = frame as usize % BUF_SIZE;
        let val = self.data.get(index)?;
        let (act_frame, val) = (*val)?;
        if act_frame != frame {
            return None;
        }
        Some(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buf() {
        let mut b: RingBuf<i32> = RingBuf::new();
        for i in 0..20 {
            assert_eq!(b.get(i), None);
        }
        for i in 0..20 {
            b.insert(i, 60 + i as i32);
        }
        // only the current frame (0) and 2 frames ahead must be inserted
        assert_eq!(b.get(0), Some(60));
        assert_eq!(b.get(1), Some(61));
        assert_eq!(b.get(2), Some(62));
        assert_eq!(b.get(3), None);

        // advance 10 frames forward
        for _ in 0..10 {
            b.advance();
        }
        assert_eq!(b.frame, 10);
        // all existing old frames must be ignored
        for i in 0..20 {
            assert_eq!(b.get(i), None);
        }
        // insert lots of frames, only the current frame, 2 before, and 2 after
        // must be inserted.
        for i in 0..20 {
            b.insert(i, 60 + i as i32);
        }
        for i in 0..=7 {
            assert_eq!(b.get(i), None);
        }
        assert_eq!(b.get(8), Some(68));
        assert_eq!(b.get(9), Some(69));
        assert_eq!(b.get(10), Some(70));
        assert_eq!(b.get(11), Some(71));
        assert_eq!(b.get(12), Some(72));
        for i in 13..=20 {
            assert_eq!(b.get(i), None);
        }
    }
}
