use super::{SCANLINES_NTSC, SCANLINES_PAL, SCANLINE_CYCLES, VIEW_HEIGHT_NTSC, VIEW_HEIGHT_PAL};
use crate::{
    cpu::{bus::AccessType, Irqs},
    schedule::{event_slots, Event, Schedule, Timestamp},
    utils::bitfield_debug,
    Model,
};

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct HvTimerIrqFlag(pub u8) {
        pub irq_triggered: bool @ 7,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HvIrqMode {
    None,
    VMatch,
    HMatch,
    VMatchHMatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counters {
    v_counter_last_change_time: Timestamp,
    v_counter: u16,
    v_display_end: u16,
    v_end: u16,
    h_irq_end_cycles: u16,
    h_end_cycles: u16,
    v_timer_value: u16,
    h_timer_value: u16,
    scheduled_hv_irq_time: Option<Timestamp>,
    hv_irq_mode: HvIrqMode,
    hv_timer_irq_flag: HvTimerIrqFlag,
}

impl Counters {
    pub(super) fn new(model: Model, schedule: &mut Schedule) -> Self {
        schedule.set_event(event_slots::HV_IRQ, Event::HvIrq);
        Counters {
            v_counter_last_change_time: 0,
            v_counter: 0,
            v_display_end: if model == Model::Pal {
                VIEW_HEIGHT_PAL
            } else {
                VIEW_HEIGHT_NTSC
            } as u16
                + 1,
            v_end: if model == Model::Pal {
                SCANLINES_PAL
            } else {
                SCANLINES_NTSC
            },
            h_irq_end_cycles: SCANLINE_CYCLES,
            h_end_cycles: SCANLINE_CYCLES,
            v_timer_value: 0x1FF,
            h_timer_value: 0x1FF,
            scheduled_hv_irq_time: None,
            hv_irq_mode: HvIrqMode::None,
            hv_timer_irq_flag: HvTimerIrqFlag(0),
        }
    }

    pub(crate) fn handle_hv_irq_triggered(&mut self, irqs: &mut Irqs, schedule: &mut Schedule) {
        self.scheduled_hv_irq_time = None;
        self.hv_timer_irq_flag.set_irq_triggered(true);
        irqs.set_hv_timer_irq_requested(true, schedule);
    }

    pub(super) fn h_dot(&self, time: Timestamp) -> u16 {
        let h_counter_cycles = (time - self.v_counter_last_change_time()) as u16;
        if self.h_end_cycles() == SCANLINE_CYCLES - 4 {
            h_counter_cycles >> 2
        } else {
            h_counter_cycles
                - (((h_counter_cycles > 323 * 4) as u16) << 1)
                - (((h_counter_cycles > 327 * 4) as u16) << 1)
        }
    }

    #[inline]
    pub fn v_counter_last_change_time(&self) -> Timestamp {
        self.v_counter_last_change_time
    }

    #[inline]
    pub fn v_counter(&self) -> u16 {
        self.v_counter
    }

    #[inline]
    pub fn v_display_end(&self) -> u16 {
        self.v_display_end
    }

    #[inline]
    pub fn v_end(&self) -> u16 {
        self.v_end
    }

    pub(super) fn start_frame(&mut self, v_display_end: u16, v_end: u16) {
        self.v_display_end = v_display_end;
        self.v_end = v_end;
    }

    #[inline]
    pub fn h_end_cycles(&self) -> u16 {
        self.h_end_cycles
    }

    #[inline]
    pub fn v_timer_value(&self) -> u16 {
        self.v_timer_value
    }

    #[inline]
    pub fn set_v_timer_value(&mut self, value: u16, time: Timestamp, schedule: &mut Schedule) {
        self.v_timer_value = value & 0x1FF;
        self.update_hv_irq(time, schedule);
    }

    #[inline]
    pub fn h_timer_value(&self) -> u16 {
        self.h_timer_value
    }

    #[inline]
    pub fn set_h_timer_value(&mut self, value: u16, time: Timestamp, schedule: &mut Schedule) {
        self.h_timer_value = value & 0x1FF;
        self.update_hv_irq(time, schedule);
    }

    #[inline]
    pub fn hv_irq_mode(&self) -> HvIrqMode {
        self.hv_irq_mode
    }

    #[inline]
    pub fn set_hv_irq_mode(&mut self, value: HvIrqMode, time: Timestamp, schedule: &mut Schedule) {
        self.hv_irq_mode = value;
        self.update_hv_irq(time, schedule);
    }

    #[inline]
    pub fn hv_timer_irq_flag(&self) -> HvTimerIrqFlag {
        self.hv_timer_irq_flag
    }

    #[inline]
    pub fn read_hv_timer_irq_flag<A: AccessType>(
        &mut self,
        irqs: &mut Irqs,
        schedule: &mut Schedule,
    ) -> HvTimerIrqFlag {
        let result = self.hv_timer_irq_flag;
        if A::SIDE_EFFECTS {
            // NOTE: The flag should actually be held for 4 cycles after being triggered, preventing
            // it from being toggled off by reads.
            self.hv_timer_irq_flag.set_irq_triggered(false);
            irqs.set_hv_timer_irq_requested(false, schedule);
        }
        result
    }

    fn irq_can_trigger(&self, h_cycles: u16) -> bool {
        if self.v_counter == self.v_end - 1 {
            // According to bsnes, the H/V IRQ can't trigger on the last dot of a frame
            h_cycles < self.h_end_cycles - 4
        } else {
            // Otherwise, just check that the requested dot is inside the possible range at all
            h_cycles < self.h_end_cycles
        }
    }

    fn h_irq_time(&self, h_timer_value: u16) -> Option<Timestamp> {
        if h_timer_value == 0 {
            // TODO: HTIME == 0 is documented to have a delay of 2.5 dots, while every other value
            // has an offset of 3.5 dots; in bsnes, all values have a delay of 3.5 dots, and only
            // testing VTIME without HTIME has a 2.5-dot delay. Which of these is correct?
            Some(self.v_counter_last_change_time + 10)
        } else {
            let h_timer_cycles = (h_timer_value << 2) + 14;
            if self.irq_can_trigger(h_timer_cycles) {
                Some(self.v_counter_last_change_time + h_timer_cycles as Timestamp)
            } else {
                None
            }
        }
    }

    fn update_hv_irq(&mut self, time: Timestamp, schedule: &mut Schedule) {
        let new_irq_time = match self.hv_irq_mode {
            HvIrqMode::None => None,
            HvIrqMode::VMatch => {
                let irq_trigger_time = time + 10;
                let h_cycles = irq_trigger_time - self.v_counter_last_change_time;
                if self.v_counter == self.v_timer_value && self.irq_can_trigger(h_cycles as u16) {
                    Some(irq_trigger_time)
                } else {
                    None
                }
            }
            HvIrqMode::VMatchHMatch => {
                if self.v_counter == self.v_timer_value {
                    self.h_irq_time(self.h_timer_value)
                } else {
                    None
                }
            }
            HvIrqMode::HMatch => self.h_irq_time(self.h_timer_value),
        };
        if new_irq_time != self.scheduled_hv_irq_time {
            if self.scheduled_hv_irq_time.is_some() {
                schedule.cancel_event(event_slots::HV_IRQ);
            }
            if let Some(new_irq_time) = new_irq_time {
                // The timer value might've been moved to point to a dot that was already passed,
                // and no IRQ should trigger at all in that case
                if new_irq_time >= time {
                    self.scheduled_hv_irq_time = Some(new_irq_time);
                    schedule.schedule_event(event_slots::HV_IRQ, new_irq_time);
                }
            } else {
                self.scheduled_hv_irq_time = None;
            }
        }
    }

    #[inline]
    pub(super) fn start_new_line(
        &mut self,
        v_counter: u16,
        h_end_cycles: u16,
        time: Timestamp,
        schedule: &mut Schedule,
    ) {
        self.v_counter = v_counter;
        // According to bsnes, the H/V IRQ can't trigger on the last dot of a frame
        self.h_irq_end_cycles = h_end_cycles - (((self.v_counter + 1 == self.v_end) as u16) << 2);
        self.h_end_cycles = h_end_cycles;
        self.v_counter_last_change_time = time;
        self.update_hv_irq(time, schedule);
    }
}
