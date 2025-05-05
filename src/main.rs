use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::time::Duration;

const GAME_WIDTH: usize = 10;
const GAME_HEIGHT: usize = 20;
const GAME_RATIO: usize = 50;
const FPS: usize = 60;

fn main() -> Result<(), String> {
    // Initialize SDL
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // Create a window
    let window = video_subsystem
        .window("Tetris", (GAME_RATIO * GAME_WIDTH) as u32, (GAME_RATIO * GAME_HEIGHT) as u32)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    // Create a canvas for rendering
    let mut canvas = window
        .into_canvas()
        .present_vsync()  // Remove this line if present
        .software()       // Force software rendering (no GPU)
        .target_texture()
        .build()
        .map_err(|e| e.to_string())?;

    // Set up event handling
    let mut event_pump = sdl_context.event_pump()?;

    //todo: change this so that state can be change by multiple threads
    let mut current_state = STATE::Tetris;
    
    // Main game loop
    'running: loop {
        // todo: add menu state
        match current_state {
            STATE::Gameover(score) => {
                // Redraw background
                canvas.set_draw_color(Color::RGB(0, 0, 0));
                canvas.clear();

                let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

                let font_path = "src/Roboto.ttf";
            
                let font = ttf_context.load_font(font_path, 24)?;
                let surface = font
                .render(&format!("Your score is: {}", score))
                .blended(Color::RGB(255, 255, 255))
                .map_err(|e| e.to_string())?;
            
                let texture_creator = canvas.texture_creator();
                let texture = texture_creator
                    .create_texture_from_surface(&surface)
                    .map_err(|e| e.to_string())?;
            
                let target = Rect::new(
                    0,
                    (GAME_HEIGHT * GAME_RATIO) as i32 / 2,
                (GAME_WIDTH * GAME_RATIO) as u32,
                40,
                );

                'gameover: loop {
                    // Now we can safely use canvas since current_game is None
                    for event in event_pump.poll_iter() {
                        match event {
                            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                                break 'running;
                            },
                            Event::KeyDown { keycode: Some(_), .. } => {
                                break 'gameover; // Exit state
                            },
                            _ => {}
                        }
                    }

                    canvas.copy(&texture, None, Some(target))?;
                    canvas.present();

                    std::thread::sleep(Duration::new(0, 1_000_000_000 / FPS as u32));   
                }

                current_state = STATE::Tetris;
            },
            STATE::Tetris => {
                // Redraw background
                canvas.set_draw_color(Color::RGB(0, 0, 0));
                canvas.clear();

                let mut game = TetrisGame::new(&mut canvas, 1221351235);

                // todo: add a substate for pausing the game
                loop {
                    for event in event_pump.poll_iter() {
                        match event {
                            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                                break 'running;
                            },
                            Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                                game.move_tetris_with_check(Position { x: -1, y: 0 })?;
                                game.canvas.present();
                            },
                            Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                                game.move_tetris_with_check(Position { x: 1, y: 0 })?;
                                game.canvas.present();
                            },
                            Event::KeyDown { keycode: Some(Keycode::X), .. } => {
                                game.rotate_tetris_left()?;
                                game.canvas.present();
                            },
                            Event::KeyDown { keycode: Some(Keycode::C), .. } => {
                                game.rotate_tetris_right()?;
                                game.canvas.present();
                            },
                            Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                                game.fast_falling = true;
                            },
                            Event::KeyUp { keycode: Some(Keycode::Down), .. } => {
                                game.fast_falling = false;
                            },
                            _ => {}
                        }
                    }

                    // Run game
                    game.update_timer()?;

                    // Exit if gameover
                    if game.gameover {
                        current_state = STATE::Gameover(game.score);
                        break;
                    }

                    std::thread::sleep(Duration::new(0, 1_000_000_000 / FPS as u32));
                }
            },
        }
    }
    
    Ok(())
}

enum STATE {
    Tetris,
    /// Stores the score
    Gameover(usize)
}

/// Basic position struct for position handeling
#[derive(Clone, Copy)]
struct Position {
    x: i32,
    y: i32
}

/// This struct contains all game logic and game states
struct TetrisGame<'a> {
    /// surface to draw on
    canvas: &'a mut Canvas<Window>,
    /// number of lines cleared
    score: usize,
    /// game grid. Uses GAME_WIDTH and GAME_HEIGHT for size
    /// 
    /// Only contains the placed squares with thier colors. Not the tetris themselves
    board: [[Option<Color>; GAME_WIDTH]; GAME_HEIGHT],
    /// Number of pixel each board squares uses
    size_ratio: u32, // Size in pixel of each square
    /// rng generator
    rng: SmallRng,
    /// Acelerates when player is pressing down
    fast_falling: bool,
    /// Number of times counter needs to increment so that a game update happens
    refresh_speed: usize,
    /// Counter for time passing
    refresh_counter: usize,
    /// Contains the struct of the current falling tetris
    current_tetris: Tetris,
    /// When true main loop will exit game
    gameover: bool
}

impl<'a> TetrisGame<'a> {
    /// Creates a new TetrisGame object
    fn new(canvas: &'a mut Canvas<Window>, seed: u64) -> Self {
        Self {
            canvas,
            score: 0,
            board: [[None; GAME_WIDTH]; GAME_HEIGHT],
            size_ratio: GAME_RATIO as u32,
            rng: rand::rngs::SmallRng::seed_from_u64(seed),
            fast_falling: false,
            refresh_speed: 20,
            refresh_counter: 0,
            current_tetris: Tetris::new(LSHAPE_RIGHT, Color { r: 255, g: 0, b: 0, a: 0 }),
            gameover: false
        }
    }

    /// Creates a new current tetris with rng
    fn make_new_random_tetris(&mut self) {
        let colors = self.rng.next_u32().to_be_bytes();
        let map: TetrisMap = match self.rng.next_u32() % 8 {
            0 => LSHAPE_RIGHT,
            1 => LSHAPE_LEFT,
            3 => ZSHAPE_LEFT,
            4 => ZSHAPE_RIGHT,
            5 => LINE,
            6 => TSHAPE,
            7 => SQUARE,
            _ => LINE
        };

        self.current_tetris = Tetris::new(map, Color::RGB(colors[0], colors[1], colors[3]));
    }

    /// Function managing game speed and fastfall
    /// 
    /// Current behavior is that the games speeds up at each 2 lines clears
    fn update_timer(&mut self) -> Result<(), String> {
        let refresh_speed = {
            if self.fast_falling {
                ((self.refresh_speed - (self.score / 4)) as f32 * 0.3) as usize
            } else {
                self.refresh_speed - (self.score / 4)
            }
        }; // speedup based on lines cleared

        self.refresh_counter += 1;
        if refresh_speed <= self.refresh_counter {
            self.update()?;
            self.refresh_counter = 0;
        }
        Ok(())
    }

    fn update(&mut self) -> Result<(), String> {

        // If at bottom insert self in game map
        if self.check_tetris_hit_bottom(&self.current_tetris, 1) || self.check_tetris_hit_board(&self.current_tetris, Position { x: 0, y: 1 }) {
            let mut cleared_line = false;
            self.insert_tetris_in_map();
            self.make_new_random_tetris();

            for (index, full) in self.full_lines().into_iter().enumerate() {
                if full {
                    self.clear_line(index);
                    cleared_line = true;
                    self.score += 1;
                }
            }

            if cleared_line {
                self.draw_refresh_all()?;
            }
            return Ok(())
        }

        self.move_tetris_with_check(Position { x: 0, y: 1 })?;
        self.canvas.present();
        Ok(())
    }

    /// Main draw function for the game grid
    /// 
    /// Draws a single square based on the position and uses ratio for each square size
    fn draw_in_grid(&mut self, postion: Position) -> Result<(), String> {
        let rectangle = Rect::new(
            postion.x * self.size_ratio as i32,
            postion.y * self.size_ratio as i32 ,
            self.size_ratio,
            self.size_ratio
        );
        return self.canvas.fill_rect(rectangle);
    }


    /// Redraws the whole game from scratch.
    /// 
    /// Avoid using this often
    fn draw_refresh_all(&mut self) -> Result<(), String> {
        self.canvas.clear(); // Erase everything

        // Redraw board
        for (yindex, y) in self.board.into_iter().enumerate() {
            for (xindex, x) in y.into_iter().enumerate() {
                if let Some(c) = x {
                    self.canvas.set_draw_color(c);
                    self.draw_in_grid(Position {x: xindex as i32, y: yindex as i32})?;
                } else {
                    self.canvas.set_draw_color(Color::RGB(0, 0, 0));
                    self.draw_in_grid(Position {x: xindex as i32, y: yindex as i32})?; 
                }
            }
        }

        self.draw_tetris()?;

        Ok(())
    }

    /// Draws current_tetris at its current position with the color of your choice
    fn draw_tetris_pick_color(&mut self, color: Color) -> Result<(), String> {
        self.canvas.set_draw_color(color);
        for (yindex, y) in self.current_tetris.map.into_iter().enumerate() {
            for (xindex, x) in y.into_iter().enumerate() {
                if x {
                    self.draw_in_grid(Position {x :(xindex as i32 + self.current_tetris.position.x), y: (yindex as i32 + self.current_tetris.position.y)})?;
                }
                
            }
        }
        Ok(())
    }

    /// Draws black current_tetris at its current position
    fn erase_tetris(&mut self) -> Result<(), String> {
        self.draw_tetris_pick_color(Color::RGB(0, 0, 0))
    }

    /// Draws current_tetris at its current position with its intended color
    fn draw_tetris(&mut self) -> Result<(), String> {
        self.draw_tetris_pick_color(self.current_tetris.color)
    }

    /// Check if tetris will hit board when shifted
    fn check_tetris_hit_board(&self, tetris: &Tetris, shift: Position) -> bool {
        let mut colided = false;
        for (yindex, y) in tetris.map.into_iter().enumerate() {
            for (xindex, x) in y.into_iter().enumerate() {
                if x &&
                    (yindex as i32 + shift.y + tetris.position.y) >= 0 &&
                    (yindex as i32 + shift.y + tetris.position.y) < GAME_HEIGHT as i32 &&
                    (xindex as i32 + shift.x + tetris.position.x) >= 0 &&
                    (xindex as i32 + shift.x + tetris.position.x) < GAME_WIDTH as i32 {
                    if self.board[(yindex as i32 + shift.y + tetris.position.y) as usize][(xindex as i32 + shift.x + tetris.position.x) as usize] != None {
                        colided = true;
                    }
                }
            }
        }

        colided
    }

    /// Check if tetris is hitting the board's walls
    fn check_tetris_hit_wall(&self, tetris: &Tetris, shift: i32) -> bool {
        tetris.position.x + tetris.most_left().unwrap_or_default() as i32 + shift < 0 ||
        tetris.position.x + tetris.most_right().unwrap_or_default() as i32 + shift >= GAME_WIDTH as i32
    }

    /// Check if tetris reached the bottom of the board
    fn check_tetris_hit_bottom(&self, tetris: &Tetris, shift: i32) -> bool {
        tetris.position.y + tetris.most_bottom().unwrap() as i32 + shift >= GAME_HEIGHT as i32
    }

    /// Move the tetris while redrawing it
    fn move_tetris(&mut self, shift: Position) -> Result<(), String> {
        self.erase_tetris()?;
        self.current_tetris.position.x += shift.x;
        self.current_tetris.position.y += shift.y;
        self.draw_tetris()?;
        Ok(())
    }

    /// Move the tetris while redrawing it
    /// 
    /// Also check if the movement will hit something and cancels if it will something
    fn move_tetris_with_check(&mut self, shift: Position) -> Result<(), String> {
        if !self.check_tetris_hit_board(&self.current_tetris, shift) && !self.check_tetris_hit_wall(&self.current_tetris, shift.x) {
            self.move_tetris(shift)?;
        }

        Ok(())
    }

    /// Rotates the tetris left
    /// 
    /// Cancels the action if it will hit something if the rotation is done
    /// 
    /// Note: If you need to rotate without checks use the tetris struct directly
    fn rotate_tetris_left(&mut self) -> Result<(), String> {
        let rotated_tetris = Tetris {
            position: self.current_tetris.position,
            map: self.current_tetris.rotate_left_result(),
            color: self.current_tetris.color
        };

        if !self.check_tetris_hit_board(&rotated_tetris, Position { x: 0, y: 0 }) && !self.check_tetris_hit_wall(&rotated_tetris, 0) {
            self.erase_tetris()?;
            self.current_tetris.rotate_left();
            self.draw_tetris()?;
        }
        
        Ok(())
    }


    /// Rotates the tetris left
    /// 
    /// Cancels the action if it will hit something if the rotation is done
    /// 
    /// Note: If you need to rotate without checks use the tetris struct directly
    fn rotate_tetris_right(&mut self) -> Result<(), String> {
        let rotated_tetris = Tetris {
            position: self.current_tetris.position,
            map: self.current_tetris.rotate_right_result(),
            color: self.current_tetris.color
        };

        if !self.check_tetris_hit_board(&rotated_tetris, Position { x: 0, y: 0 }) && !self.check_tetris_hit_wall(&rotated_tetris, 0) {
            self.erase_tetris()?;
            self.current_tetris.rotate_right();
            self.draw_tetris()?;
        }
        
        Ok(())
    }

    /// Insert current_tetris inside board
    fn insert_tetris_in_map(&mut self) {
        for (yindex, y) in self.current_tetris.map.into_iter().enumerate() {
            for (xindex, x) in y.into_iter().enumerate() {
                if x &&
                    yindex as i32 + self.current_tetris.position.y >= 0 &&
                    yindex as i32 + self.current_tetris.position.y < GAME_HEIGHT as i32 &&
                    xindex as i32 + self.current_tetris.position.x >= 0 &&
                    xindex as i32 + self.current_tetris.position.x < GAME_WIDTH as i32 {
                    self.board[(yindex as i32 + self.current_tetris.position.y) as usize][(xindex as i32 + self.current_tetris.position.x) as usize] = Some(self.current_tetris.color);
                }

                if x && yindex as i32 + self.current_tetris.position.y < 0 {
                    self.gameover = true;
                }
            }
        } 
    }


    /// Finds all of the full lines in boards.
    /// 
    /// Return an array with the false or true for each line.
    /// True if the line is full
    /// 
    /// Important to have it like this because only give the full line indexes would use HEAP
    fn full_lines(&self) -> [bool; GAME_HEIGHT] {
        let mut full_lines = [false; GAME_HEIGHT];
        for (index, line) in self.board.into_iter().enumerate() {
            if line.into_iter().find(|x| x == &None) == None {
                full_lines[index] = true;
            }
        }

        full_lines
    }

    /// Deletes the specified line and shift all lines above it
    fn clear_line(&mut self, line_index: usize) {
        // Shift lines todo: dont use clone here
        for (index, line) in self.board.clone()[0..line_index].into_iter().enumerate() {
            self.board[index + 1] = line.clone();
        }
        self.board[0] = [None; GAME_WIDTH]; // Remove top line
    }
}

/// Map type to make sure tetris management stays consistent
type TetrisMap = [[bool; 5]; 5];

const LINE: TetrisMap = [[false, false, false, false, false],
                        [false, false, false, false, false],
                        [false, true, true, true, true],
                        [false, false, false, false, false],
                        [false, false, false, false, false]];
const LSHAPE_RIGHT: TetrisMap = [[false, false, false, false, false],
                                [false, false, false, true, false],
                                [false, true, true, true, false],
                                [false, false, false, false, false],
                                [false, false, false, false, false]];
const LSHAPE_LEFT: TetrisMap = [[false, false, false, false, false],
                                [false, false, false, false, false],
                                [false, true, true, true, false],
                                [false, false, false, true, false],
                                [false, false, false, false, false]];
const ZSHAPE_RIGHT: TetrisMap = [[false, false, false, false, false],
                                [false, true, true, false, false],
                                [false, false, true, true, false],
                                [false, false, false, false, false],
                                [false, false, false, false, false]];

const ZSHAPE_LEFT: TetrisMap = [[false, false, false, false, false],
                                [false, false, true, true, false],
                                [false, true, true, false, false],
                                [false, false, false, false, false],
                                [false, false, false, false, false]];

const TSHAPE: TetrisMap = [[false, false, false, false, false],
                            [false, false, true, false, false],
                            [false, true, true, true, false],
                            [false, false, false, false, false],
                            [false, false, false, false, false]];

const SQUARE: TetrisMap = [[false, false, false, false, false],
                                [false, false, true, true, false],
                                [false, false, true, true, false],
                                [false, false, false, false, false],
                                [false, false, false, false, false]];

/// Contains the tetris props and transform logics
struct Tetris {
    /// Current tetris position
    position: Position,
    /// Contains the tetris shape
    map: TetrisMap,
    /// Tetris color
    color: Color
}

impl Tetris {
    /// Makes a new tetris object
    fn new(map: TetrisMap, color: Color) -> Self {
        Self {
            position: Position {x: (GAME_WIDTH as i32 / 2) - 2, y: -5},
            map,
            color
        }
    }

    /// Return the tetris map if it was rotated right
    fn rotate_right_result(&self) -> TetrisMap {
        let mut new_tetris: TetrisMap = [[false; 5]; 5];
        
        for (yindex, y) in self.map.iter().enumerate() {
            for (xindex, x) in y.iter().enumerate() {
                new_tetris[xindex][4 - yindex] = x.clone();
            }
        }

        return new_tetris;
    }

    /// Return the tetris map if it was rotated left
    fn rotate_left_result(&self) -> TetrisMap {
        let mut new_tetris: TetrisMap = [[false; 5]; 5];
        
        for (yindex, y) in self.map.iter().enumerate() {
            for (xindex, x) in y.iter().enumerate() {
                new_tetris[4 - xindex][yindex] = x.clone();
            }
        }

        return new_tetris;
    }

    /// Rotates the tetris map right directly
    fn rotate_right(&mut self) {
        self.map = self.rotate_right_result();
    }

    /// Rotates the tetris map left directly
    fn rotate_left(&mut self) {
        self.map = self.rotate_left_result();
    }

    /// Finds the most left square in map
    /// 
    /// returns the x index of it
    fn most_left(&self) -> Option<usize> {
        let mut pos: Option<usize> = None;
        for y in self.map.iter() {
            for (xindex, x) in y.iter().enumerate() {
                if x.clone() && xindex < pos.unwrap_or(200) {
                    pos = Some(xindex);
                }
            }
        }

        pos
    }

    /// Finds the most right square in map
    /// 
    /// returns the x index of it
    fn most_right(&self) -> Option<usize> {
        let mut pos: Option<usize> = None;
        for y in self.map.iter() {
            for (xindex, x) in y.iter().enumerate() {
                if x.clone() && xindex > pos.unwrap_or(0) {
                    pos = Some(xindex);
                }
            }
        }

        pos
    }

    /// Finds the most bottom square in map
    /// 
    /// returns the y index of it
    fn most_bottom(&self) -> Option<usize> {
        let mut pos: Option<usize> = None;
        for (yindex, y) in self.map.iter().enumerate() {
            if y.contains(&true) {
                pos = Some(yindex);
            }
        }

        pos
    }
}