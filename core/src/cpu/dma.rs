use super::bus;
use crate::utils::bitfield_debug;
use crate::{emu::Emu, schedule::Schedule};

mod bounded {
    use crate::utils::bounded_int;
    bounded_int!(pub struct Index(u8), max 7);
}
pub use bounded::Index;

bitfield_debug! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct ChannelControl(pub u8) {
        pub transfer_unit: u8 @ 0..=2,
        pub gp_addr_step: u8 @ 3..=4,
        pub h_indirect: bool @ 6,
        pub direction: bool @ 7,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Channel {
    control: ChannelControl,
    gp_addr_step: i8,
    h_do_transfer: bool,
    pub b_addr: u8,
    pub gp_a_addr_h_table_start_addr: u16,
    pub gp_a_bank_h_table_bank: u8,
    pub gp_byte_counter_h_indirect_addr: u16,
    pub h_indirect_bank: u8,
    pub h_cur_table_addr: u16,
    h_line_counter: u8,
    pub unused: u8,
}

impl Channel {
    #[inline]
    pub fn control(&self) -> ChannelControl {
        self.control
    }

    #[inline]
    pub fn set_control(&mut self, value: ChannelControl) {
        self.control = value;
        self.gp_addr_step = match value.gp_addr_step() {
            0 => 1,
            2 => -1,
            _ => 0,
        };
    }

    #[inline]
    pub fn h_line_counter(&self) -> u8 {
        self.h_line_counter
    }

    #[inline]
    pub fn set_h_line_counter(&mut self, value: u8) {
        self.h_line_counter = if value & 0x7F == 0 {
            // TODO: What happens?
            value + 1
        } else {
            value
        };
    }

    #[inline]
    pub fn gp_addr_step(&self) -> i8 {
        self.gp_addr_step
    }
}

pub struct Controller {
    pub channels: [Channel; 8],
    gp_requested: u8,
    h_enabled: u8,
    h_frame_enabled: u8,
    h_requested: u8,
    pub cur_channel: Option<Index>,
}

impl Controller {
    pub(crate) fn new() -> Self {
        Controller {
            channels: [Channel {
                control: ChannelControl(0xFF),
                gp_addr_step: 0,
                h_do_transfer: false,
                b_addr: 0xFF,
                gp_a_addr_h_table_start_addr: 0xFFFF,
                gp_a_bank_h_table_bank: 0xFF,
                gp_byte_counter_h_indirect_addr: 0xFFFF,
                h_indirect_bank: 0,
                h_cur_table_addr: 0xFFFF,
                h_line_counter: 0xFF,
                unused: 0xFF,
            }; 8],
            gp_requested: 0,
            h_enabled: 0,
            h_frame_enabled: 0,
            h_requested: 0,
            cur_channel: None,
        }
    }

    fn select_next_channel(&mut self) {
        let requested = (self.gp_requested as u16) << 8 | self.h_requested as u16;
        if requested == 0 {
            return;
        }
        let first_channel = Index::new(requested.trailing_zeros() as u8 & 7);
        if if let Some(channel) = self.cur_channel {
            first_channel < channel
        } else {
            true
        } {
            self.cur_channel = Some(first_channel);
        }
    }

    #[inline]
    pub fn gp_requested(&self) -> u8 {
        self.gp_requested
    }

    #[inline]
    pub fn set_gp_requested(&mut self, value: u8, schedule: &mut Schedule) {
        self.gp_requested = value;
        self.select_next_channel();
        schedule.target_time = schedule.cur_time;
    }

    #[inline]
    pub fn h_enabled(&self) -> u8 {
        self.h_enabled
    }

    #[inline]
    pub fn set_h_enabled(&mut self, value: u8) {
        self.h_enabled = value;
        self.h_frame_enabled &= value;
    }

    fn reload_hdma_data<const INITIAL: bool>(emu: &mut Emu, i: Index) {
        let channel = &emu.cpu.dmac.channels[i.get() as usize];
        let mut table_addr = channel.h_cur_table_addr;
        let table_bank_base = (channel.gp_a_bank_h_table_bank as u32) << 16;
        let counter_value = bus::read::<bus::CpuAccess>(emu, table_bank_base | table_addr as u32);
        emu.schedule.cur_time += 8;
        table_addr = table_addr.wrapping_add(1);

        let channel = &mut emu.cpu.dmac.channels[i.get() as usize];
        if channel.control.h_indirect() {
            let mut addr_low =
                bus::read::<bus::CpuAccess>(emu, table_bank_base | table_addr as u32);
            emu.schedule.cur_time += 8;
            table_addr = table_addr.wrapping_add(1);

            let addr_high =
                if !INITIAL && counter_value == 0 && emu.cpu.dmac.h_requested == 1 << i.get() {
                    let res = addr_low;
                    addr_low = 0;
                    res
                } else {
                    let result =
                        bus::read::<bus::CpuAccess>(emu, table_bank_base | table_addr as u32);
                    emu.schedule.cur_time += 8;
                    table_addr = table_addr.wrapping_add(1);
                    result
                };

            emu.cpu.dmac.channels[i.get() as usize].gp_byte_counter_h_indirect_addr =
                (addr_high as u16) << 8 | addr_low as u16;
        }

        let channel = &mut emu.cpu.dmac.channels[i.get() as usize];
        channel.h_cur_table_addr = table_addr;
        channel.h_do_transfer = counter_value != 0;
        if channel.h_do_transfer {
            channel.h_line_counter = counter_value;
        } else {
            emu.cpu.dmac.h_frame_enabled &= !(1 << i.get());
        }
    }

    pub(crate) fn reload_hdmas(emu: &mut Emu) {
        // TODO: What if an event fires during HDMA reloading?

        if emu.cpu.dmac.h_enabled == 0 {
            return;
        }

        emu.schedule.cur_time += 18;
        emu.cpu.dmac.h_frame_enabled = emu.cpu.dmac.h_enabled;

        for i in 0..8 {
            if emu.cpu.dmac.h_enabled & 1 << i == 0 {
                continue;
            }

            let channel = &mut emu.cpu.dmac.channels[i];
            channel.h_cur_table_addr = channel.gp_a_addr_h_table_start_addr;

            Self::reload_hdma_data::<true>(emu, Index::new(i as u8));
        }
    }

    pub(crate) fn start_hdmas(&mut self) {
        self.h_requested |= self.h_frame_enabled;
        self.select_next_channel();
    }

    pub(crate) fn run_dma(emu: &mut Emu, i: Index) {
        // TODO: Same as above, a lot of events could happen mid-transfer

        macro_rules! transfer {
            (
                @all_inner,
                $channel: ident,
                $get_a_addr: expr,
                $after_transfer: expr,
                $(
                    $($transfer_unit: literal)|* => [$($get_b_addr: expr),*$(,)?]
                ),*$(,)?
            ) => {{
                let channel = &emu.cpu.dmac.channels[i.get() as usize];
                if channel.control.direction() {
                    match channel.control.transfer_unit() {
                        $(
                            $($transfer_unit)|* => {
                                $(
                                    let $channel = &mut emu.cpu.dmac.channels[i.get() as usize];
                                    let a_addr = $get_a_addr;
                                    let b_addr = $get_b_addr;
                                    let value = bus::read_b_io::<bus::DmaAccess>(emu, b_addr);
                                    bus::write::<bus::DmaAccess>(emu, a_addr, value);
                                    emu.schedule.cur_time += 8;
                                    $after_transfer;
                                )*
                            }
                        ),*,
                        _ => unreachable!(),
                    }
                } else {
                    match channel.control.transfer_unit() {
                        $(
                            $($transfer_unit)|* => {
                                $(
                                    let $channel = &mut emu.cpu.dmac.channels[i.get() as usize];
                                    let a_addr = $get_a_addr;
                                    let b_addr = $get_b_addr;
                                    let value = bus::read::<bus::DmaAccess>(emu, a_addr);
                                    bus::write_b_io::<bus::DmaAccess>(emu, b_addr, value);
                                    emu.schedule.cur_time += 8;
                                    $after_transfer;
                                )*
                            }
                        ),*,
                        _ => unreachable!(),
                    }
                }
            }};

            (
                $channel: ident,
                $get_a_addr: expr,
                $after_transfer: expr$(,)?
            ) => {{
                transfer!(
                    @all_inner,
                    $channel,
                    $get_a_addr,
                    $after_transfer,
                    0 => [$channel.b_addr],
                    1 => [$channel.b_addr, $channel.b_addr.wrapping_add(1)],
                    2 | 6 => [$channel.b_addr, $channel.b_addr],
                    3 | 7 => [
                        $channel.b_addr,
                        $channel.b_addr,
                        $channel.b_addr.wrapping_add(1),
                        $channel.b_addr.wrapping_add(1),
                    ],
                    4 => [
                        $channel.b_addr,
                        $channel.b_addr.wrapping_add(1),
                        $channel.b_addr.wrapping_add(2),
                        $channel.b_addr.wrapping_add(3),
                    ],
                    5 => [
                        $channel.b_addr,
                        $channel.b_addr.wrapping_add(1),
                        $channel.b_addr,
                        $channel.b_addr.wrapping_add(1),
                    ],
                )
            }};
        }

        if emu.cpu.dmac.h_requested & 1 << i.get() != 0 {
            let channel = &emu.cpu.dmac.channels[i.get() as usize];
            if channel.h_do_transfer {
                transfer!(
                    channel,
                    if channel.control.h_indirect() {
                        channel.gp_byte_counter_h_indirect_addr as u32
                            | (channel.h_indirect_bank as u32) << 16
                    } else {
                        channel.h_cur_table_addr as u32
                            | (channel.gp_a_bank_h_table_bank as u32) << 16
                    },
                    {
                        let channel = &mut emu.cpu.dmac.channels[i.get() as usize];
                        if channel.control.h_indirect() {
                            channel.gp_byte_counter_h_indirect_addr =
                                channel.gp_byte_counter_h_indirect_addr.wrapping_add(1);
                        } else {
                            channel.h_cur_table_addr = channel.h_cur_table_addr.wrapping_add(1);
                        }
                    },
                );
            }
            let channel = &mut emu.cpu.dmac.channels[i.get() as usize];
            channel.h_line_counter -= 1;
            if channel.h_line_counter & 0x7F == 0 {
                Self::reload_hdma_data::<false>(emu, i);
            } else {
                channel.h_do_transfer = channel.h_line_counter & 0x80 != 0;
            }

            emu.cpu.dmac.h_requested &= !(1 << i.get());
            emu.cpu.dmac.cur_channel = None;
            emu.cpu.dmac.select_next_channel();
        } else {
            loop {
                transfer!(
                    channel,
                    channel.gp_a_addr_h_table_start_addr as u32
                        | (channel.gp_a_bank_h_table_bank as u32) << 16,
                    {
                        let channel = &mut emu.cpu.dmac.channels[i.get() as usize];
                        channel.gp_a_addr_h_table_start_addr = channel
                            .gp_a_addr_h_table_start_addr
                            .wrapping_add(channel.gp_addr_step as u16);
                        channel.gp_byte_counter_h_indirect_addr =
                            channel.gp_byte_counter_h_indirect_addr.wrapping_sub(1);
                        if channel.gp_byte_counter_h_indirect_addr == 0 {
                            emu.cpu.dmac.gp_requested &= !(1 << i.get());
                            emu.cpu.dmac.cur_channel = None;
                            emu.cpu.dmac.select_next_channel();
                            break;
                        }
                    },
                );
            }
        }
    }
}
