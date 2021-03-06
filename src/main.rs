const HZ: u32 = 700;
const DHZ: u32 = 700;

struct CompatibilityOptions {
    shift_ignores_vy: bool,
    no_increment: bool,
    jump_table_variant: bool,
}

impl Default for CompatibilityOptions {
    fn default() -> Self {
        Self {
            shift_ignores_vy: false,
            no_increment: false,
            jump_table_variant: false,
        }
    }
}

type Addr = usize;

struct Chip {
    opts: CompatibilityOptions,
    mem: Vec<u8>,
    pc: Addr,
    ir: Addr,
    stack: Vec<Addr>,
    dt: u8,
    st: u8,
    regs: [u8; 16],
    screen: [[bool; 64]; 32],
    muted: bool,
}

impl Chip {
    fn new(rom: Vec<u8>, opts: CompatibilityOptions) -> Self {
        let mut mem: Vec<u8> = [0; 4096].into();
        mem[0x50..0xA0].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]);
        mem[0x200..(0x200 + rom.len())].copy_from_slice(&rom);
        Self {
            opts,
            mem,
            pc: 0x200,
            ir: 0,
            stack: Vec::new(),
            dt: 0,
            st: 0,
            regs: [0; 16],
            screen: [[false; 64]; 32],
            muted: false,
        }
    }

    fn step(&mut self, input: &winit_input_helper::WinitInputHelper) -> bool {
        let b1 = self.mem[self.pc];
        let b2 = self.mem[self.pc + 1];
        let opcode: u8 = (b1 & 0xf0) >> 4;
        let x: u8 = b1 & 0x0f;
        let y: u8 = (b2 & 0xf0) >> 4;
        let n: u8 = b2 & 0x0f;
        let nn: u8 = b2;
        let nnn: usize = (((b1 & 0x0f) as u16 * 256) | (b2 as u16)) as usize;
        if !self.muted {
            print!(
                "[{:03x}] {:02X}{:02X} op={:X} x={:X} y={:X} n={:X} nn={:02X} nnn={:03X} ",
                self.pc, b1, b2, opcode, x, y, n, nn, nnn
            )
        }
        self.pc += 2;
        let mut desc: Option<String> = None;
        let mut drew = false;
        match opcode {
            0x0 => {
                if nnn == 0x0e0 {
                    desc = Some("clear screen".to_owned());
                    self.screen = [[false; 64]; 32];
                } else if nnn == 0x0ee {
                    desc = Some("return".to_owned());
                    match self.stack.pop() {
                        None => {
                            println!("error: stack underflow (return instruction skipped")
                        }
                        Some(addr) => self.pc = addr,
                    }
                }
            }
            0x1 => {
                desc = Some(format!("jump to {:03X}", nnn));
                if self.pc == nnn + 2 {
                    self.muted = true
                }
                self.pc = nnn;
            }
            0x2 => {
                desc = Some(format!("call subroutine at {:03X}", nnn));
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            0x3 => {
                desc = Some(format!("skip if register {:X} equals {:02X}", x, nn));
                if self.regs[x as usize] == nn {
                    self.pc += 2;
                }
            }
            0x4 => {
                desc = Some(format!(
                    "skip if register {:X} does not equal {:02X}",
                    x, nn
                ));
                if self.regs[x as usize] != nn {
                    self.pc += 2;
                }
            }
            0x5 => {
                desc = Some(format!("skip if register {:X} equals register {:X}", x, y));
                if self.regs[x as usize] == self.regs[y as usize] {
                    self.pc += 2;
                }
            }
            0x6 => {
                desc = Some(format!("set register {:X} to {:02X}", x, nn));
                self.regs[x as usize] = nn;
            }
            0x7 => {
                desc = Some(format!("increase register {:X} by {:02X}", x, nn));
                let r = &mut self.regs[x as usize];
                *r = r.wrapping_add(nn);
            }
            0x8 => match n {
                0x0 => {
                    desc = Some(format!("set register {:X} to value in register {:X}", x, y));
                    self.regs[x as usize] = self.regs[y as usize];
                }
                0x1 => {
                    desc = Some(format!(
                        "OR register {:X} with value in register {:X}",
                        x, y
                    ));
                    self.regs[x as usize] |= self.regs[y as usize];
                }
                0x2 => {
                    desc = Some(format!(
                        "AND register {:X} with value in register {:X}",
                        x, y
                    ));
                    self.regs[x as usize] &= self.regs[y as usize];
                }
                0x3 => {
                    desc = Some(format!(
                        "XOR register {:X} with value in register {:X}",
                        x, y
                    ));
                    self.regs[x as usize] ^= self.regs[y as usize];
                }
                0x4 => {
                    desc = Some(format!(
                        "Increase register {:X} by value in register {:X}",
                        x, y
                    ));
                    let result = self.regs[x as usize] as u16 + self.regs[y as usize] as u16;
                    self.regs[x as usize] = (result & 0xff) as u8;
                    self.regs[0xf] = if result > 0xff { 1 } else { 0 };
                }
                0x5 | 0x7 => {
                    let (m, s) = if n == 0x5 {
                        desc = Some(format!(
                            "Subtract register {:X} from register {:X} and store in register {:X}",
                            y, x, x
                        ));
                        (self.regs[x as usize], self.regs[y as usize])
                    } else {
                        desc = Some(format!(
                            "Subtract register {:X} from register {:X} and store in register {:X}",
                            x, y, x
                        ));
                        (self.regs[y as usize], self.regs[x as usize])
                    };
                    self.regs[x as usize] = m.wrapping_sub(s);
                    self.regs[0xf] = if s > m { 0 } else { 1 };
                }
                0x6 => {
                    let v = if self.opts.shift_ignores_vy {
                        desc = Some(format!("Shift register {:X} right (*)", x));
                        self.regs[x as usize]
                    } else {
                        desc = Some(format!(
                            "Shift register {:X} right and store in register {:X} (*)",
                            y, x
                        ));
                        self.regs[x as usize]
                    };
                    let flag = v & 0x1;
                    self.regs[x as usize] = v >> 1;
                    self.regs[0xf] = flag;
                }
                0xe => {
                    let v = if self.opts.shift_ignores_vy {
                        desc = Some(format!("Shift register {:X} left (*)", x));
                        self.regs[x as usize]
                    } else {
                        desc = Some(format!(
                            "Shift register {:X} left and store in register {:X} (*)",
                            y, x
                        ));
                        self.regs[x as usize]
                    };
                    let flag = (v & 0b10000000) >> 7;
                    self.regs[x as usize] = (v << 1) & 0xff;
                    self.regs[0xf] = flag;
                }
                _ => (),
            },
            0x9 => {
                desc = Some(format!(
                    "skip if register {:X} does not equal register {:X}",
                    x, y
                ));
                if self.regs[x as usize] != self.regs[y as usize] {
                    self.pc += 2;
                }
            }
            0xA => {
                desc = Some(format!("set index register to {:03X}", nnn));
                self.ir = nnn;
            }
            0xB => {
                let offs = if self.opts.jump_table_variant {
                    desc = Some(format!(
                        "jump by table at {:03X} using value in register {:X} (*)",
                        nnn, x
                    ));
                    self.regs[x as usize]
                } else {
                    desc = Some(format!(
                        "jump by table at {:03X} using value in register 0 (*)",
                        nnn
                    ));
                    self.regs[0]
                };
                let dest = nnn + offs as usize;
                if dest == self.pc - 2 {
                    self.muted = true
                }
                self.pc = dest;
            }
            0xC => {
                let result: u8 = rand::random::<u8>() & nn;
                desc = Some(format!(
                    "store random number with mask {:02X} in register {:X} (result: {:02X})",
                    nn, x, result
                ));
                self.regs[x as usize] = result;
            }
            0xD => {
                desc = Some(format!(
                    "draw {} rows with X={:X}, Y={:X} ({},{})",
                    n, x, y, self.regs[x as usize], self.regs[y as usize]
                ));
                drew = true;
                let px = (self.regs[x as usize] % 64) as usize;
                let py = (self.regs[y as usize] % 32) as usize;
                self.regs[0xf] = 0;
                for dy in 0..(n as usize) {
                    if py + dy >= 32 {
                        break;
                    }
                    let data = self.mem[self.ir + dy];
                    for dx in 0..8 {
                        if px + dx >= 64 {
                            break;
                        }
                        let pixel = &mut self.screen[py + dy][px + dx];
                        let draw = (data >> (7 - dx)) & 1 == 1;
                        if *pixel && draw {
                            self.regs[0xf] = 1;
                        }
                        *pixel = *pixel ^ draw;
                    }
                }
            }
            0xE => {
                let keycode = get_key_code(self.regs[x as usize]);
                if nn == 0x9E {
                    desc = Some(format!(
                        "skip if key in register {:X} ({:?}) is pressed",
                        x, keycode
                    ));
                    if input.key_held(keycode) {
                        self.pc += 2
                    }
                } else if nn == 0xA1 {
                    desc = Some(format!(
                        "skip if key in register {:X} ({:?}) is not pressed",
                        x, keycode
                    ));
                    if !input.key_held(keycode) {
                        self.pc += 2
                    }
                }
            }
            0xF => {
                if nn == 0x07 {
                    desc = Some(format!("store delay timer in register {:X}", x));
                    self.regs[x as usize] = self.dt;
                } else if nn == 0x0A {
                    desc = Some(format!("wait for input and save key in register {:X}", x));
                    let mut pressed = false;
                    for k in 0..16 {
                        if input.key_held(get_key_code(k)) {
                            self.regs[x as usize] = k;
                            pressed = true;
                            break;
                        }
                    }
                    if !pressed {
                        self.pc -= 2;
                    }
                } else if nn == 0x15 {
                    desc = Some(format!("set delay timer to value in register {:X}", x));
                    self.dt = self.regs[x as usize];
                } else if nn == 0x18 {
                    desc = Some(format!("set sound timer to value in register {:X}", x));
                    self.st = self.regs[x as usize];
                } else if nn == 0x1E {
                    desc = Some(format!("add value in register {:X} to index register", x));
                    let sum = self.ir as u16 + self.regs[x as usize] as u16;
                    self.ir = (sum & 0xfff) as Addr;
                    self.regs[0xf] = ((sum & 0x1000) >> 12) as u8;
                } else if nn == 0x29 {
                    desc = Some(format!(
                        "set index register to address of character for register {:X}",
                        x
                    ));
                    let val = (self.regs[x as usize] & 0xf) as usize;
                    self.ir = 0x50 + val * 5;
                } else if nn == 0x33 {
                    desc = Some(format!("write BCD value in register {:X} to memory", x));
                    let val = self.regs[x as usize];
                    self.mem[self.ir] = val / 100;
                    self.mem[self.ir + 1] = (val % 100) / 10;
                    self.mem[self.ir + 2] = val % 10;
                } else if nn == 0x55 {
                    desc = Some(format!("store values in first {} registers to memory", x));
                    for i in 0..=(x as usize) {
                        self.mem[self.ir + i] = self.regs[i];
                    }
                    if !self.opts.no_increment {
                        self.ir += x as usize;
                    }
                } else if nn == 0x65 {
                    desc = Some(format!(
                        "load values from first {} registers into memory",
                        x
                    ));
                    for i in 0..=(x as usize) {
                        self.regs[i] = self.mem[self.ir + i];
                    }
                    if !self.opts.no_increment {
                        self.ir += x as usize;
                    }
                }
            }
            _ => (),
        }
        match desc {
            None => println!("unknown opcode"),
            Some(d) => {
                if !self.muted {
                    println!("{}", d)
                }
            }
        }
        drew
    }
}

use winit::event::VirtualKeyCode;

fn get_key_code(n: u8) -> VirtualKeyCode {
    match n {
        0 => VirtualKeyCode::X,
        1 => VirtualKeyCode::Key1,
        2 => VirtualKeyCode::Key2,
        3 => VirtualKeyCode::Key3,
        4 => VirtualKeyCode::Q,
        5 => VirtualKeyCode::W,
        6 => VirtualKeyCode::E,
        7 => VirtualKeyCode::A,
        8 => VirtualKeyCode::S,
        9 => VirtualKeyCode::D,
        0xA => VirtualKeyCode::Z,
        0xB => VirtualKeyCode::C,
        0xC => VirtualKeyCode::Key4,
        0xD => VirtualKeyCode::R,
        0xE => VirtualKeyCode::F,
        0xF => VirtualKeyCode::V,
        _ => unreachable!(),
    }
}

fn main() {
    let mut opts = CompatibilityOptions::default();
    let mut a = std::env::args().skip(1);
    let filename = match a.next() {
        None => panic!("expected at least one argument"),
        Some(arg) => arg,
    };
    let mut quiet = false;
    for arg in a {
        if arg == "--quiet" {
            println!("Quiet mode");
            quiet = true;
        } else if arg == "--shift" {
            println!("Super-Chip compatibility: 8XY6 and 8XYE ignore their second operand");
            opts.shift_ignores_vy = true;
        } else if arg == "--jump" {
            println!(
                "Super-Chip compatibility: BNNN uses VX rather than V0 for the jump table index"
            );
            opts.jump_table_variant = true;
        } else if arg == "--no-inc" {
            println!("Super-Chip compatibility: FX55 and FX65 do not modify the index register");
            opts.no_increment = true;
        } else {
            panic!("unknown argument")
        }
    }
    let bytes = std::fs::read(filename).expect("could not read ROM file");
    let mut chip = Chip::new(bytes, opts);
    chip.muted = quiet;
    println!("{} bytes in memory", chip.mem.len());

    let event_loop = winit::event_loop::EventLoop::new();
    let mut input = winit_input_helper::WinitInputHelper::new();
    let window = {
        let size = winit::dpi::LogicalSize::new(256, 128);
        winit::window::WindowBuilder::new()
            .with_title("CHIP8")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture =
            pixels::SurfaceTexture::new(window_size.width, window_size.height, &window);
        pixels::Pixels::new(64, 32, surface_texture).unwrap()
    };
    let start = std::time::Instant::now();
    let mut spent = std::time::Duration::from_secs(0);
    let mut timer_delay = std::time::Duration::from_secs(0);
    event_loop.run(move |event, _, control_flow| {
        if let winit::event::Event::RedrawRequested(_) = event {
            for (y, row) in pixels.get_frame().chunks_exact_mut(64 * 4).enumerate() {
                for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
                    let c = if chip.screen[y][x] { 0xff } else { 0x11 };
                    pixel.copy_from_slice(&[0, c, 0, 0]);
                }
            }
            if pixels.render().is_err() {
                *control_flow = winit::event_loop::ControlFlow::Exit;
                return;
            }
        }
        if input.update(&event) {
            if input.key_pressed(winit::event::VirtualKeyCode::Escape) || input.quit() {
                *control_flow = winit::event_loop::ControlFlow::Exit;
                return;
            }
        }
        if let Some(size) = input.window_resized() {
            pixels.resize_surface(size.width, size.height);
        }
        let mut redraw = false;
        while std::time::Instant::now().duration_since(start) > spent {
            let t;
            if chip.step(&input) {
                t = std::time::Duration::from_secs(1) / HZ;
                redraw = true;
            } else {
                t = std::time::Duration::from_secs(1) / DHZ;
            }
            spent += t;
            timer_delay += t;
        }
        let frame = std::time::Duration::from_secs_f32(1. / 60.);
        while timer_delay > frame {
            timer_delay = timer_delay.saturating_sub(frame);
            if chip.dt > 0 {
                chip.dt -= 1
            }
            if chip.st > 0 {
                chip.st -= 1
            }
        }
        if redraw {
            window.request_redraw();
        }
    })
}
