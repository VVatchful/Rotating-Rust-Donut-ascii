use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use std::{
    error::Error,
    io::{stdout, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use std::f32::consts::PI;

// Function to get the terminal size
fn get_terminal_size() -> (usize, usize) {
    // Retrieve the terminal size or set a default size if unsuccessful
    let (width, height) = size().unwrap_or((240, 80));
    (width as usize, height as usize)
}

// Function to render each frame of the ASCII donut
fn render_frame(a: f32, b: f32, width: usize, height: usize) -> Result<(), Box<dyn Error>> {
    // Initialize the output buffer and z-buffer
    let mut output = vec![' '; width * height];
    let mut zbuffer = vec![f32::INFINITY; width * height];

    // Constants and pre-calculated values for rendering
    let theta_spacing = 0.07;
    let phi_spacing = 0.02;
    let r1 = 1.0;
    let r2 = 2.0;
    let k2 = 5.0;
    let k1 = width as f32 * k2 * 3.0 / (8.0 * (r1 + r2));  // Adjusted dynamically to width

    let (sin_a, cos_a) = a.sin_cos();
    let (sin_b, cos_b) = b.sin_cos();

    // Loop through each point in the 3D space and calculate its projection onto the 2D screen
    for theta in (0..).map(|i| i as f32 * theta_spacing).take_while(|&theta| theta < 2.0 * PI) {
        let (sin_theta, cos_theta) = theta.sin_cos();
        for phi in (0..).map(|i| i as f32 * phi_spacing).take_while(|&phi| phi < 2.0 * PI) {
            let (sin_phi, cos_phi) = phi.sin_cos();
            let circle_x = r2 + r1 * cos_theta;
            let x = circle_x * (cos_b * cos_phi + sin_a * sin_b * sin_phi) - r1 * cos_a * sin_b * sin_phi;
            let y = circle_x * sin_phi * cos_b - r1 * sin_b * sin_phi * cos_a;
            let z = k2 + cos_a * circle_x * sin_phi + sin_a * r1 * cos_phi;
            let ooz = k1 / z;
            let xp = (width as f32 / 2.0 + x * ooz) as isize;
            let yp = (height as f32 / 2.0 - y * ooz) as isize;
            if xp >= 0 && xp < width as isize && yp >= 0 && yp < height as isize {
                let index = xp as usize + width * yp as usize;
                let l = cos_phi * cos_theta * sin_b - cos_a * cos_theta * sin_phi - sin_a * sin_theta + cos_b * (cos_a * sin_theta - cos_theta * sin_a * sin_phi);
                if l > 0.0 && z < zbuffer[index] {  // Depth test
                    zbuffer[index] = z;
                    let luminance_index = (l * 12.0).clamp(0.0, 11.0).floor() as usize;
                    output[index] = ".,-~:;=!*#$@%&"[luminance_index..=luminance_index].chars().next().unwrap();
                }
            }
        }
    }

    // Clear the screen and print the ASCII art
    print!("\x1B[H");
    for k in 0..(width * height) {
        if k % width == 0 {
            print!("\n");
        }
        print!("{}", output[k]);
    }
    stdout().flush()?;  // Flush stdout to ensure output is displayed immediately
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize terminal and get its size
    let (width, height) = get_terminal_size();
    let (tx, rx) = mpsc::channel();  // Create a channel for inter-thread communication
    let handle = thread::spawn(move || {
        // Thread to handle keyboard input
        loop {
            if let Ok(true) = event::poll(Duration::from_millis(100)) {
                if let Ok(Event::Key(key_event)) = event::read() {
                    tx.send(key_event).unwrap();  // Send key events to the main thread
                }
            }
        }
    });

    // Initialize parameters for the donut animation
    let mut a = 0.0_f32;
    let mut b = 0.0_f32;
    let mut a_speed = 0.04;
    let mut b_speed = 0.08;
    let mut running = true;

    execute!(stdout(), EnterAlternateScreen)?;  // Switch to alternate screen buffer
    terminal::enable_raw_mode()?;  // Enable raw mode to handle keyboard input

    // Main loop for rendering frames and handling input
    while running {
        let now = Instant::now();  // Record current time
        // Process keyboard input
        while let Ok(KeyEvent { code, .. }) = rx.try_recv() {
            match code {
                KeyCode::Up => a_speed += 0.01,
                KeyCode::Down => a_speed -= 0.01,
                KeyCode::Right => b_speed += 0.01,
                KeyCode::Left => b_speed -= 0.01,
                KeyCode::Char('r') => { a_speed = 0.04; b_speed = 0.08; },  // Reset speeds
                KeyCode::Char('p') => { a_speed = 0.0; b_speed = 0.0; },  // Pause animation
                KeyCode::Char('s') => { a_speed = 0.04; b_speed = 0.08; },  // Start animation
                KeyCode::Esc => { running = false; break; },  // Exit program
                _ => {}
            }
        }

        // Render the current frame of the donut animation
        render_frame(a, b, width, height)?;
        // Update parameters for next frame
        a += a_speed;
        b += b_speed;
        let elapsed = now.elapsed();  // Calculate time elapsed since the start of the frame
        let delay = Duration::from_millis(50).saturating_sub(elapsed);  // Calculate delay to maintain 20 FPS
        thread::sleep(delay);  // Wait for the remaining time to maintain frame rate
    }

    // Clean up: disable raw mode, switch back to main screen buffer, and join the keyboard input thread
    terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    handle.join().ok();

    Ok(())
}

