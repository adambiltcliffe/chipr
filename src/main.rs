const HZ: u32 = 700;
const DHZ: u32 = 60;

type Addr = usize;

struct Chip {
    mem: Vec<u8>,
    pc: Addr,
    ir: Addr,
    stack: Vec<Addr>,
    dt: u8,
    st: u8,
    regs: [u8; 16],
    screen: [[bool; 64]; 32],
}

impl Chip {
    fn new(rom: Vec<u8>) -> Self {
        let mut mem: Vec<u8> = [0; 0x200].into();
        mem.extend(rom);
        Self {
            mem,
            pc: 0x200,
            ir: 0,
            stack: Vec::new(),
            dt: 0,
            st: 0,
            regs: [0; 16],
            screen: [[false; 64]; 32],
        }
    }

    fn step(&mut self) -> bool {
        let b1 = self.mem[self.pc];
        let b2 = self.mem[self.pc + 1];
        let opcode: u8 = (b1 & 0xf0) >> 4;
        let x: u8 = b1 & 0x0f;
        let y: u8 = (b2 & 0xf0) >> 4;
        let n: u8 = b2 & 0x0f;
        let nn: u8 = b2;
        let nnn: usize = (((b1 & 0x0f) as u16 * 256) | (b2 as u16)) as usize;
        print!(
            "[{:03x}] {:02X}{:02X} op={:X} x={:X} y={:X} n={:X} nn={:02X} nnn={:03X} ",
            self.pc, b1, b2, opcode, x, y, n, nn, nnn
        );
        self.pc += 2;
        let mut desc: Option<String> = None;
        let mut drew = false;
        match opcode {
            0x0 => {
                if nnn == 0x0e0 {
                    desc = Some("clear screen".to_owned());
                    self.screen = [[false; 64]; 32];
                }
            }
            0x1 => {
                desc = Some(format!("jump to {:03X}", nnn).to_owned());
                self.pc = nnn;
            }
            0x6 => {
                desc = Some(format!("set register {:X} to {:02X}", x, nn));
                self.regs[x as usize] = nn;
            }
            0x7 => {
                desc = Some(format!("increase register {:X} by {:02X}", x, nn));
                let r = &mut self.regs[x as usize];
                *r = (*r + nn) % 0xff;
            }
            0xA => {
                desc = Some(format!("set index register to {:03X}", nnn));
                self.ir = nnn;
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
                        *pixel = (data >> (7 - dx)) & 1 == 1;
                    }
                }
            }
            _ => (),
        }
        match desc {
            None => println!("unknown opcode"),
            Some(d) => println!("{}", d),
        }
        drew
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        panic!("expected a single argument")
    }
    let bytes = std::fs::read(&args[1]).expect("could not read ROM file");
    let mut chip = Chip::new(bytes);
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
            if chip.step() {
                spent += std::time::Duration::from_secs(1) / HZ;
                redraw = true;
            } else {
                spent += std::time::Duration::from_secs(1) / DHZ;
            }
        }
        if redraw {
            window.request_redraw();
        }
    })
}