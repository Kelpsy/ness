// TODO: IRQ delay emulation, especially interacting with WAI and DMAs

use crate::schedule::Schedule;

#[derive(Debug)]
pub struct Irqs {
    irqs_enabled: bool,
    waiting_for_exception: bool,
    hv_timer_irq_requested: bool,
    processing_irq: bool,
    processing_nmi: bool,
}

impl Irqs {
    pub(super) fn new() -> Self {
        Irqs {
            irqs_enabled: true,
            waiting_for_exception: false,
            hv_timer_irq_requested: false,
            processing_irq: false,
            processing_nmi: false,
        }
    }

    #[inline]
    pub fn irqs_enabled(&self) -> bool {
        self.irqs_enabled
    }

    fn update_irqs(&mut self, schedule: &mut Schedule) {
        self.processing_irq = self.hv_timer_irq_requested && self.irqs_enabled;
        if self.processing_irq {
            schedule.set_target_to_cur();
        }
    }

    #[inline]
    pub fn set_irqs_enabled(&mut self, value: bool, schedule: &mut Schedule) {
        self.irqs_enabled = value;
        self.update_irqs(schedule);
    }

    #[inline]
    pub fn waiting_for_exception(&self) -> bool {
        self.waiting_for_exception
    }

    #[inline]
    pub fn set_waiting_for_exception(&mut self, value: bool) {
        self.waiting_for_exception = value && !(self.processing_nmi || self.hv_timer_irq_requested);
    }

    #[inline]
    pub fn hv_timer_irq_requested(&self) -> bool {
        self.hv_timer_irq_requested
    }

    #[inline]
    pub fn set_hv_timer_irq_requested(&mut self, value: bool, schedule: &mut Schedule) {
        self.hv_timer_irq_requested = value;
        self.waiting_for_exception &= !value;
        self.update_irqs(schedule);
    }

    #[inline]
    pub fn processing_irq(&self) -> bool {
        self.processing_irq
    }

    #[inline]
    pub fn processing_nmi(&self) -> bool {
        self.processing_nmi
    }

    #[inline]
    pub fn request_nmi(&mut self, schedule: &mut Schedule) {
        self.processing_nmi = true;
        schedule.set_target_to_cur();
        self.waiting_for_exception = false;
    }

    pub(super) fn acknowledge_nmi(&mut self) {
        self.processing_nmi = false;
    }
}
