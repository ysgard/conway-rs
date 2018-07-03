extern crate tcod;

use tcod::{Console, FontLayout, FontType, Renderer, RootConsole};
use tcod::system;
use tcod::input;
use tcod::colors as color;
use tcod::noise;

use std::thread;
use std::time::{Duration, Instant};

const MAP_WIDTH: usize = 300;
const MAP_HEIGHT: usize = 80;
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 40;
const FPS: i32 = 25;

// Chosen purely because it looks good
const NOISE_VERT: f32 = 12.0;
const NOISE_HORI: f32  = 40.0;

#[derive(Copy, Clone)]
struct Cell {
    alive: bool,
    linger: u8,
    flip: bool
}

struct Map {
    map: [[Cell; MAP_HEIGHT]; MAP_WIDTH],
    height: usize,
    width: usize,
    o_x: i32,
    o_y: i32
}

#[derive(PartialEq)]
enum GameState {
    Initializing,
    Running,
    Ending
}

impl Map {
    fn new() -> Map {
        Map {
            map: [[ Cell { alive: false, linger: 0, flip: false}; MAP_HEIGHT]
                  ; MAP_WIDTH],
            height: MAP_HEIGHT,
            width: MAP_WIDTH,
            o_x: (MAP_WIDTH as i32 - SCREEN_WIDTH) / 2,
            o_y: (MAP_HEIGHT as i32 - SCREEN_HEIGHT) / 2
        }
    }


    fn inc_linger(&mut self, x: usize, y: usize) {
        if self.map[x][y].linger < 9 { self.map[x][y].linger += 1; }
    }

    fn dec_linger(&mut self, x: usize, y: usize) {
        if self.map[x][y].linger > 0 { self.map[x][y].linger -= 1; }
    }
    
    // debug function
    #[allow(dead_code)]
    fn live_cells(&self) -> i32 {
        self.map.iter().flat_map(|r| r.iter())
            .filter(|cell| cell.alive == true)
            .count() as i32
    }

    fn live_neighbours(&self, x: usize, y: usize) -> i32 {
        let mut count = 0;
        let h = self.height - 1;
        let w = self.width - 1;
            
        let li = if x == 0 { 0 } else { x - 1 };
        let lj = if y == 0 { 0 } else { y - 1 };
        let hi = if x == w { w } else { x + 1 };
        let hj = if y == h { h } else { y + 1 };
        for i in li..(hi + 1) {
            for j in lj..(hj + 1) {
                if i == x && j == y {
                    continue;
                } else {
                    if self.map[i][j].alive { count += 1 };
                }
            }
        }
        count
    }

    fn live_die(&mut self, x: usize, y: usize) -> bool {
        // Is it alive?
        let n = self.live_neighbours(x, y);
        if self.map[x][y].alive {
            // Check to see if it dies
            if n > 3 || n < 2 {
                self.flip_one(x, y, true)
            } else { false }
        } else {
            // It's dead.  Does it live?
            if n == 3 {
                self.flip_one(x, y, true)
            } else { false }
        }
    }
            
    fn flip_one(&mut self, x: usize, y: usize, flip: bool) -> bool {
        self.map[x][y].flip = flip;
        flip
    }
                

    fn flip_all(&mut self) {
        for x in 0..(self.width - 1) {
            for y in 0..(self.height - 1) {
                let mut cell = &mut self.map[x as usize][y as usize];
                if cell.flip {
                    cell.alive = !cell.alive;
                    cell.flip = false;
                }
            }
        }
    }

    fn init_noise(&mut self) {
        let noise2d = noise::Noise::init_with_dimensions(2).init();
        let mut p: [f32; 2] = [ 0.0, 0.0 ];
        for x in 0..(self.width - 1) {
            for y in 0..(self.height - 1) {
                p[0] = (x as f32 * NOISE_HORI) / self.width as f32;
                p[1] = (y as f32 * NOISE_VERT) / self.height as f32;
                let noise = noise2d.get_ex(p, noise::NoiseType::Perlin);
                if noise >= 0.0 { self.map[x as usize][y as usize].alive = true };
            }
        }
    }

    fn toggle(&mut self, x: i32, y: i32) {
        let i = x as usize;
        let j = y as usize;
        self.map[i][j].alive = !self.map[i][j].alive;
    }

    fn tick(&mut self) {
        // flip cells depending on the rules
        for i in 0..self.width - 1 {
            for j in 0..self.height - 1 {
                self.live_die(i, j);
            }
        }
        // Cascade the flips into live/dead cells.  The reason we toggle a flip
        // flag before this point is that we don't want cells toggled earlier
        // in the array to affect cells further in the array.
        self.flip_all();
        // Update linger values.  Live cells brighten, dead cells fade.
        for i in 0..self.width - 1 {
            for j in 0..self.height - 1 {
                if self.map[i][j].alive {
                    // Grow to a maximum of 9
                    self.inc_linger(i, j);
                } else {
                    // Fade to a minimum of 0
                    self.dec_linger(i, j);
                }
            }
        }
    }
}


fn display_map(root: &mut Console, map: &Map) {
    let color_scale = [
        color::BLACK,
        color::DARKEST_RED,
        color::DARKER_RED,
        color::DARK_RED,
        color::RED,
        color::FLAME,
        color::ORANGE,
        color::AMBER,
        color::YELLOW,
        color::LIGHT_YELLOW
    ];
    for x in 0..SCREEN_WIDTH {
        for y in 0..SCREEN_HEIGHT {
            let cell = &map.map[(x + map.o_x) as usize][(y + map.o_y) as usize];
            let c = if cell.alive { '*' } else { ' ' };
            root.put_char_ex(x, y, c, color::WHITE, color_scale[cell.linger as usize]);
        }
    }
}

fn main() {

    let mut map = Map::new();
    map.init_noise();
    
    // Initialize tcod
    let mut root = RootConsole::initializer()
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("conway-rs")
        .font("BrogueFont3.png", FontLayout::AsciiInRow)
        .font_type(FontType::Greyscale)
        .renderer(Renderer::SDL)
        .init();

    // Clamp FPS
    system::set_fps(FPS);

    // Declare game loop variables;
    let mut game_state = GameState::Initializing;
    let frame_time = Duration::from_millis(1000 / (FPS as u64));
    

    // Main loop
    while game_state != GameState::Ending && !root.window_closed() {

        let start_time = Instant::now();
        
        display_map(&mut root, &map);
        root.flush();
  
        match input::check_for_event(input::KEY | input::MOUSE) {
            None => {},
            Some((_, event)) => {
                match event {
                    input::Event::Key(ref key_state) => {
                        if key_state.code == input::KeyCode::Enter && key_state.pressed {
                            game_state = match game_state {
                                GameState::Initializing => GameState::Running,
                                GameState::Running => GameState::Initializing,
                                GameState::Ending => GameState::Ending
                            };
                        if key_state.code == input::KeyCode::Escape { game_state = GameState::Ending };
                        }
                    },
                    input::Event::Mouse(ref mouse_state) => {
                        let x = mouse_state.cx as i32 + map.o_x;
                        let y = mouse_state.cy as i32 + map.o_y;
                        if mouse_state.lbutton_pressed { map.toggle(x, y) };                    }
                }
            }
        }
        if game_state == GameState::Running { map.tick() }

        // Wait until a full frame time has elapsed
        let time_diff = start_time.elapsed();
        if time_diff < frame_time {
            thread::sleep(frame_time - time_diff);
        }
    }
}
