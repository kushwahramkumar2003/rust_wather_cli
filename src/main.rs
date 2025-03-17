use chrono::{TimeZone, Utc};
use colored::Colorize;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::process;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "weather", about = "A weather CLI application")]
struct Opt {
    #[structopt(short, long)]
    city: Option<String>,

    #[structopt(short, long)]
    fahrenheit: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = match env::var("OPEN_WEATHER_MAP_API") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("{}",
                "Error: OPEN_WEATHER_MAP_API environment variable not set. Please add it to your .env file."
                .bright_red()
            );
            process::exit(1);
        }
    };

    let opt = Opt::from_args();

    if let Some(city) = opt.city {
        match get_and_display_weather(&city, &api_key, opt.fahrenheit).await {
            Ok(_) => {}
            Err(e) => eprintln!("{} {}", "Error:".bright_red(), e),
        }
    } else {
        // Interactive mode
        println!("{}", "🌤️  Weather CLI v1.0".bold());
        println!("{}", "Enter 'q' or 'exit' to quit".italic());

        loop {
            let city = get_input("Enter city name:").await;

            if city.to_lowercase() == "q" || city.to_lowercase() == "exit" {
                println!("👋 Goodbye!");
                break;
            }

            match get_and_display_weather(&city, &api_key, opt.fahrenheit).await {
                Ok(_) => {}
                Err(e) => eprintln!("{} {}", "Error:".bright_red(), e),
            }

            println!(); // Add a newline for better readability
        }
    }

    Ok(())
}

async fn get_input(input_msg: &str) -> String {
    let mut input = String::new();
    println!("{} ", input_msg.bright_cyan());
    std::io::stdin().read_line(&mut input).unwrap_or_default();
    input.trim().to_string()
}

async fn get_and_display_weather(
    city: &str,
    api_key: &str,
    use_fahrenheit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match get_city_weather(city, api_key).await {
        Ok(weather) => {
            display_weather(&weather, use_fahrenheit);
            Ok(())
        }
        Err(e) => Err(format!("Failed to get weather data for '{}': {}", city, e).into()),
    }
}

async fn get_city_weather(
    city: &str,
    api_key: &str,
) -> Result<WeatherData, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let res = client
        .get("https://api.openweathermap.org/data/2.5/weather")
        .query(&[("q", city), ("appid", api_key), ("units", "metric")])
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        if status.as_u16() == 404 {
            return Err(format!("City '{}' not found", city).into());
        } else {
            return Err(format!("API error: HTTP {}", status).into());
        }
    }

    let weather_data = res.json::<WeatherData>().await?;
    Ok(weather_data)
}

fn display_weather(weather: &WeatherData, use_fahrenheit: bool) {
    println!(
        "\n{}",
        "═════════════════════════════════════════".bright_yellow()
    );
    println!(
        "🌍 {} {}, {}",
        "Weather in".bright_green(),
        weather.name.bold(),
        weather.sys.country.bold()
    );

    // Weather condition
    let weather_icon = get_weather_emoji(&weather.weather[0].main);
    println!(
        "{} {} ({})",
        weather_icon,
        weather.weather[0].main.bold(),
        weather.weather[0].description
    );

    // Temperature
    let temp = if use_fahrenheit {
        format!("{:.1}°F", celsius_to_fahrenheit(weather.main.temp))
    } else {
        format!("{:.1}°C", weather.main.temp)
    };

    let feels_like = if use_fahrenheit {
        format!("{:.1}°F", celsius_to_fahrenheit(weather.main.feels_like))
    } else {
        format!("{:.1}°C", weather.main.feels_like)
    };

    println!(
        "🌡️ Temperature: {} (feels like {})",
        temp.bright_yellow(),
        feels_like
    );

    // Min/Max temps
    let temp_min = if use_fahrenheit {
        format!("{:.1}°F", celsius_to_fahrenheit(weather.main.temp_min))
    } else {
        format!("{:.1}°C", weather.main.temp_min)
    };

    let temp_max = if use_fahrenheit {
        format!("{:.1}°F", celsius_to_fahrenheit(weather.main.temp_max))
    } else {
        format!("{:.1}°C", weather.main.temp_max)
    };

    println!("📊 Min/Max: {}/{}", temp_min, temp_max);

    // Humidity and pressure
    println!("💧 Humidity: {}%", weather.main.humidity);
    println!("🔄 Pressure: {} hPa", weather.main.pressure);

    // Wind
    println!(
        "💨 Wind: {:.1} m/s, Direction: {}°",
        weather.wind.speed, weather.wind.deg
    );

    if let Some(gust) = weather.wind.gust {
        println!("🌬️ Gusts: {:.1} m/s", gust);
    }

    // Visibility
    println!("👁️ Visibility: {} km", weather.visibility / 1000);

    // Clouds
    println!("☁️ Cloudiness: {}%", weather.clouds.all);

    // Sunrise & Sunset
    let sunrise = format_timestamp(weather.sys.sunrise, weather.timezone);
    let sunset = format_timestamp(weather.sys.sunset, weather.timezone);
    println!("🌅 Sunrise: {}", sunrise);
    println!("🌇 Sunset: {}", sunset);

    println!(
        "{}",
        "═════════════════════════════════════════".bright_yellow()
    );
}

fn get_weather_emoji(condition: &str) -> &'static str {
    match condition.to_lowercase().as_str() {
        "clear" => "☀️",
        "thunderstorm" => "⛈️",
        "drizzle" => "🌦️",
        "rain" => "🌧️",
        "snow" => "❄️",
        "mist" | "smoke" | "haze" | "dust" | "fog" | "sand" | "ash" | "squall" => "🌫️",
        "clouds" => "☁️",
        "tornado" => "🌪️",
        _ => "🌤️",
    }
}

fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    (celsius * 9.0 / 5.0) + 32.0
}

fn format_timestamp(timestamp: i64, timezone_offset: i32) -> String {
    let datetime = Utc.timestamp_opt(timestamp, 0).unwrap();
    let local_time =
        datetime.with_timezone(&chrono::FixedOffset::east_opt(timezone_offset).unwrap());
    local_time.format("%H:%M:%S").to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeatherData {
    pub coord: Coord,
    pub weather: Vec<Weather>,
    pub base: String,
    pub main: Main,
    pub visibility: i32,
    pub wind: Wind,
    pub clouds: Clouds,
    pub dt: i64,
    pub sys: Sys,
    pub timezone: i32,
    pub id: i64,
    pub name: String,
    pub cod: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Coord {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Weather {
    pub id: i32,
    pub main: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Main {
    pub temp: f64,
    pub feels_like: f64,
    pub temp_min: f64,
    pub temp_max: f64,
    pub pressure: i32,
    pub humidity: i32,
    pub sea_level: Option<i32>,
    pub grnd_level: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Wind {
    pub speed: f64,
    pub deg: i32,
    pub gust: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Clouds {
    pub all: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sys {
    pub country: String,
    pub sunrise: i64,
    pub sunset: i64,
}
