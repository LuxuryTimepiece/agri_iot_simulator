use rand::Rng;
use tokio::time::{sleep, Duration};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use std::io;

/// Represents the possible states of the soil moisture sensor
#[derive(Debug, PartialEq)]
enum DeviceState {
    Monitoring,  // Checking environmental conditions
    Activating,  // Initiating watering
    Adjusting,   // Watering in progress
    Idle,        // Optimal moisture level
    Error,       // System error
}

/// Simulates a soil moisture sensor with state and animation tracking
struct SoilMoistureSensor {
    state: DeviceState,
    moisture_level: f32, // Percentage (0.0 to 100.0)
    threshold: f32,      // Water if below this level
    just_watered: bool,  // Prevents immediate moisture drop after watering
    animation_frame: usize, // Tracks blinking frames (0 or 1)
}

impl SoilMoistureSensor {
    /// Creates a new sensor with a given moisture threshold
    fn new(threshold: f32) -> Self {
        Self {
            state: DeviceState::Monitoring,
            moisture_level: 50.0,
            threshold,
            just_watered: false,
            animation_frame: 0,
        }
    }

    /// Transitions the sensor state based on moisture levels
    async fn transition(&mut self, new_moisture: f32) -> Option<String> {
        if !self.just_watered {
            self.moisture_level = new_moisture.max(0.0);
        }
        self.just_watered = false;
        let message = match self.state {
            DeviceState::Monitoring => {
                self.animation_frame = (self.animation_frame + 1) % 2;
                if self.moisture_level < self.threshold {
                    self.state = DeviceState::Activating;
                    self.animation_frame = 0;
                    Some(format!("Moisture low ({:.1}%), activating...", self.moisture_level))
                } else {
                    None
                }
            }
            DeviceState::Activating => {
                self.moisture_level += 15.0;
                self.state = DeviceState::Adjusting;
                self.just_watered = true;
                self.animation_frame = (self.animation_frame + 1) % 2;
                Some(format!("Watering... Moisture now {:.1}%", self.moisture_level))
            }
            DeviceState::Adjusting => {
                self.animation_frame = 0;
                if self.moisture_level >= self.threshold + 10.0 {
                    self.state = DeviceState::Idle;
                    Some(format!("Moisture optimal ({:.1}%), going idle", self.moisture_level))
                } else if self.moisture_level < self.threshold {
                    self.state = DeviceState::Monitoring;
                    Some(format!("Moisture still low ({:.1}%), back to monitoring", self.moisture_level))
                } else {
                    None
                }
            }
            DeviceState::Idle => {
                self.animation_frame = 0;
                if self.moisture_level < self.threshold {
                    self.state = DeviceState::Monitoring;
                    Some("Moisture dropping, back to monitoring".to_string())
                } else {
                    None
                }
            }
            DeviceState::Error => {
                self.animation_frame = (self.animation_frame + 1) % 2;
                Some("Error state, no transitions".to_string())
            }
        };
        sleep(Duration::from_secs(1)).await;
        message
    }
}

static FLOWER_BASE: &str = "            .--. 
      .-\"-:`    `:-\"-.
   .-/     '.  .'     \\-.
  ;__|      _::_      |__;
 /`   '.  /` \\/` \\  .'   `\\
 |      _ \\      / _      |
 \\    /` '.'.  .'.' `\\    /
/ '-._'.  _'./\\.'_  .'_.-' \\
\\ .-' .'`  .'\\//'.  `'. '-. /
 /    \\._.'.'  '.'._./    \\
 |        /      \\        |
 \\.__ .'  \\._/\\_.//  '. __./
  ;  |       ::       |  ;
   '-\\     .'  '.     /-'
 jgs  '-.-:_    _:-.-'
            '--'";

// Add lifetime 'a to tie Line to the borrowed line
fn style_line<'a>(index: usize, line: &'a str, blink_frame: usize, state: &DeviceState) -> Line<'a> {
    match state {
        DeviceState::Monitoring => Line::from(line).style(Style::default().fg(Color::Yellow)),
        DeviceState::Activating => {
            let color = if blink_frame == 0 { Color::Blue } else { Color::Black };
            Line::from(line).style(Style::default().fg(color))
        }
        DeviceState::Adjusting => Line::from(line).style(Style::default().fg(Color::Cyan)),
        DeviceState::Idle => {
            if index == 11 || (index >= 7 && index <= 9 && line.contains(" / ")) {
                Line::from(line).style(Style::default().fg(Color::White)) // Center
            } else {
                Line::from(line).style(Style::default().fg(Color::Rgb(255, 165, 0))) // Petals
            }
        }
        DeviceState::Error => {
            let color = if blink_frame == 0 { Color::Red } else { Color::Black };
            Line::from(line).style(Style::default().fg(color))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut sensor = SoilMoistureSensor::new(30.0);
    let mut rng = rand::thread_rng();
    let mut status_message = String::new();

    loop {
        let drop = rng.gen_range(0.5..2.0);
        let new_moisture = (sensor.moisture_level - drop).max(0.0);
        if let Some(msg) = sensor.transition(new_moisture).await {
            status_message = msg;
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.size());

            let text = format!(
                "State: {:?}\nMoisture: {:.1}%\nStatus: {}",
                sensor.state, sensor.moisture_level, status_message
            );
            let status = Paragraph::new(text)
                .block(Block::default().title("Agri-IoT Simulator").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            f.render_widget(status, chunks[0]);

            let animation_lines: Vec<Line> = FLOWER_BASE
                .lines()
                .enumerate()
                .map(|(i, line)| style_line(i, line, sensor.animation_frame, &sensor.state))
                .collect();
            let animation = Paragraph::new(Text::from(animation_lines))
                .block(Block::default().title("Neon Flower").borders(Borders::ALL));
            f.render_widget(animation, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
                // Optional: Press 'e' to trigger Error state
                if key.code == KeyCode::Char('e') {
                    sensor.state = DeviceState::Error;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}