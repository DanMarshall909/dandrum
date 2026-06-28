use crate::patch::VoiceStealingPolicy;

#[derive(Clone, Debug, PartialEq)]
pub struct VoiceSlot {
    pub active: bool,
    pub note: u8,
    pub velocity: u8,
    allocation_order: u64,
}

#[derive(Clone, Debug)]
pub struct VoiceAllocator {
    slots: Vec<VoiceSlot>,
    max_voices: usize,
    stealing: VoiceStealingPolicy,
    next_order: u64,
}

impl VoiceAllocator {
    pub fn new(max_voices: u32, stealing: VoiceStealingPolicy) -> Self {
        let max = max_voices.max(1) as usize;
        let slots = (0..max)
            .map(|_| VoiceSlot {
                active: false,
                note: 0,
                velocity: 0,
                allocation_order: 0,
            })
            .collect();
        Self {
            slots,
            max_voices: max,
            stealing,
            next_order: 1,
        }
    }

    pub fn max_voices(&self) -> usize {
        self.max_voices
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) -> Option<usize> {
        let order = self.next_order;
        self.next_order += 1;

        if let Some(free) = self.free_slot() {
            self.slots[free] = VoiceSlot {
                active: true,
                note,
                velocity,
                allocation_order: order,
            };
            return Some(free);
        }

        if self.stealing == VoiceStealingPolicy::OldestActive {
            let steal = self.oldest_active_slot();
            self.slots[steal] = VoiceSlot {
                active: true,
                note,
                velocity,
                allocation_order: order,
            };
            return Some(steal);
        }

        None
    }

    pub fn note_off(&mut self, note: u8) {
        for slot in &mut self.slots {
            if slot.active && slot.note == note {
                slot.active = false;
            }
        }
    }

    pub fn active_count(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.active)
    }

    pub fn slot(&self, index: usize) -> Option<&VoiceSlot> {
        self.slots.get(index)
    }

    pub fn set_slot_inactive(&mut self, slot: usize) {
        if let Some(s) = self.slots.get_mut(slot) {
            s.active = false;
            s.note = 0;
            s.velocity = 0;
        }
    }

    fn free_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| !s.active)
    }

    fn oldest_active_slot(&self) -> usize {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.active)
            .min_by_key(|(_, s)| s.allocation_order)
            .map(|(i, _)| i)
            .expect("oldest_active_slot called when no slots active")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_notes_use_independent_voice_slots() {
        let mut alloc = VoiceAllocator::new(3, VoiceStealingPolicy::Disabled);

        let s0 = alloc.note_on(60, 100).expect("should allocate slot 0");
        let s1 = alloc.note_on(64, 100).expect("should allocate slot 1");

        assert_ne!(s0, s1);
        assert_eq!(alloc.active_count(), 2);
        assert_eq!(alloc.slot(s0).unwrap().note, 60);
        assert_eq!(alloc.slot(s1).unwrap().note, 64);
    }

    #[test]
    fn note_off_releases_only_matching_note() {
        let mut alloc = VoiceAllocator::new(3, VoiceStealingPolicy::Disabled);

        alloc.note_on(60, 100);
        alloc.note_on(64, 100);
        alloc.note_off(60);

        assert_eq!(alloc.active_count(), 1);
    }

    #[test]
    fn repeated_same_note_allocates_separate_voices_until_full() {
        let mut alloc = VoiceAllocator::new(3, VoiceStealingPolicy::Disabled);

        let s0 = alloc.note_on(60, 100).expect("slot 0");
        let s1 = alloc.note_on(60, 100).expect("slot 1");
        let s2 = alloc.note_on(60, 100).expect("slot 2");

        assert_ne!(s0, s1);
        assert_ne!(s1, s2);
        assert_eq!(alloc.active_count(), 3);
        assert!(alloc.is_full());
    }

    #[test]
    fn oldest_active_voice_is_stolen_when_full() {
        let mut alloc = VoiceAllocator::new(2, VoiceStealingPolicy::OldestActive);

        alloc.note_on(60, 100);
        alloc.note_on(64, 100);
        // Full now — next note steals oldest (60)
        let stolen = alloc.note_on(67, 100).expect("should steal oldest");

        assert_eq!(stolen, 0);
        assert_eq!(alloc.active_count(), 2);
        assert_eq!(alloc.slot(0).unwrap().note, 67);
        assert_eq!(alloc.slot(1).unwrap().note, 64);
    }

    #[test]
    fn full_allocator_without_stealing_ignores_new_note() {
        let mut alloc = VoiceAllocator::new(2, VoiceStealingPolicy::Disabled);

        alloc.note_on(60, 100);
        alloc.note_on(64, 100);
        let result = alloc.note_on(67, 100);

        assert!(result.is_none());
        assert_eq!(alloc.active_count(), 2);
    }

    #[test]
    fn allocator_creation_always_produces_at_least_one_slot() {
        let alloc = VoiceAllocator::new(0, VoiceStealingPolicy::Disabled);
        assert_eq!(alloc.max_voices(), 1);
    }

    #[test]
    fn note_off_releases_only_first_matching_note_regardless_of_note_identity() {
        let mut alloc = VoiceAllocator::new(3, VoiceStealingPolicy::Disabled);

        alloc.note_on(60, 100);
        alloc.note_on(60, 100);
        alloc.note_on(64, 100);
        alloc.note_off(60);

        // Two notes were note 60, so both should be released
        assert_eq!(alloc.active_count(), 1);
        assert_eq!(alloc.slot(2).unwrap().note, 64);
    }
}
