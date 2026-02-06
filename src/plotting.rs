use rgb::RGB8;
use serde::{Deserialize, Serialize};
use termize::dimensions;
use textplots::{Chart, ColorPlot, Plot, Shape};

use crate::{config::CONFIG, utils::bool_from_env};

/// High-level plotting helper
pub struct ChartBuilder {
    config: ChartConfig,
    plots: Vec<Vec<(f32, f32)>>,
}

impl ChartBuilder {
    pub fn new() -> Self {
        Self {
            config: ChartConfig::new(),
            plots: Vec::new(),
        }
    }

    pub fn new_with_dimensions(width: u32, height: u32) -> Self {
        Self {
            config: ChartConfig::new_with_dimensions(width, height),
            plots: Vec::new(),
        }
    }

    pub fn add_plot(&mut self, points: Vec<(f32, f32)>) -> &mut Self {
        self.plots.push(points);
        self
    }

    /// Calculate and render the chart
    pub fn build_and_display(&mut self) {
        self.config.width = (self.config.width as f32 * self.config.scale) as u32;
        self.config.height =
            self.config.width * self.config.aspect_ratio.1 / self.config.aspect_ratio.0;

        let (xmin, xmax, ymin, ymax) = self.infer_bounds();

        let mut horizontal: Vec<f32> = Vec::new();
        let mut vertical: Vec<(f32, f32, f32)> = Vec::new();

        if self.config.borders {
            horizontal.push(ymax);
            horizontal.push(ymin);
            vertical.push((xmin, ymin, ymax));
            vertical.push((xmax, ymin, ymax));
        }
        if self.config.lines {
            let start = ymin.ceil() as i32;
            let end = ymax.floor() as i32;
            for y in start..=end {
                horizontal.push(y as f32);
            }
        }

        let bg_line_buffers: Vec<Vec<(f32, f32)>> = vertical
            .iter()
            .map(|(x, y1, y2)| vec![(*x, *y1), (*x, *y2)])
            .collect();

        let mut bg_shapes: Vec<Shape> = Vec::new();

        horizontal
            .iter()
            .map(|y| Shape::Continuous(Box::new(move |_| *y)))
            .for_each(|s| bg_shapes.push(s));

        bg_line_buffers
            .iter()
            .map(|line| Shape::Lines(line))
            .for_each(|s| bg_shapes.push(s));

        let grade_plots: Vec<Shape<'_>> =
            self.plots.iter().map(|line| Shape::Lines(line)).collect();

        let mut chart = Chart::new_with_y_range(
            self.config.width,
            self.config.height,
            xmin,
            xmax,
            ymin,
            ymax,
        );

        let chart = if self.config.use_color {
            Self::apply_color_shapes(&mut chart, &bg_shapes, RGB8::from(self.config.bg_color))
        } else {
            Self::apply_shapes(&mut chart, &bg_shapes)
        };
        let chart = if self.config.use_color {
            Self::apply_color_shapes(chart, &grade_plots, RGB8::from(self.config.plot_color))
        } else {
            Self::apply_shapes(chart, &grade_plots)
        };

        chart.figures();
        print!("{chart}");
    }

    /// Infer bounds from points
    fn infer_bounds(&self) -> (f32, f32, f32, f32) {
        let xmin = 0f32;
        let xmax = self
            .plots
            .iter()
            .map(|points| points.len())
            .min()
            .unwrap_or(0) as f32;
        let ymin = 1f32;
        let ymax = 5f32;

        if self.config.padding {
            let x_span = (xmax - xmin).max(1.0);
            let x_pad = x_span * 0.02; // NOTE magicnumber
            return (xmin - x_pad, xmax + x_pad, ymin, ymax);
        }

        (xmin, xmax, ymin, ymax)
    }

    fn apply_shapes<'a>(chart: &'a mut Chart<'a>, shapes: &'a [Shape<'a>]) -> &'a mut Chart<'a> {
        if let Some((shape, rest)) = shapes.split_first() {
            let chart = chart.lineplot(shape);
            return Self::apply_shapes(chart, rest);
        }
        chart
    }

    fn apply_color_shapes<'a>(
        chart: &'a mut Chart<'a>,
        shapes: &'a [Shape<'a>],
        color: RGB8,
    ) -> &'a mut Chart<'a> {
        if let Some((shape, rest)) = shapes.split_first() {
            let chart = chart.linecolorplot(shape, color);
            return Self::apply_color_shapes(chart, rest, color);
        }
        chart
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartConfig {
    #[serde(skip)]
    width: u32,
    #[serde(skip)]
    height: u32,
    aspect_ratio: (u32, u32),
    scale: f32,
    borders: bool,
    lines: bool,
    use_color: bool,
    plot_color: (u8, u8, u8),
    bg_color: (u8, u8, u8),
    padding: bool,
}

impl ChartConfig {
    pub fn new() -> ChartConfig {
        let mut new_chartcfg = Self::default();

        new_chartcfg.check_app_config();
        new_chartcfg.check_env();
        new_chartcfg
    }
    pub fn new_with_dimensions(width: u32, height: u32) -> ChartConfig {
        let mut new_chartcfg = Self::default();

        new_chartcfg.check_app_config();
        new_chartcfg.check_env();
        new_chartcfg
    }

    fn check_env(&mut self) {
        bool_from_env("RSFILC_CHARTS_BORDER", &mut self.borders);
        bool_from_env("RSFILC_CHARTS_LINES", &mut self.lines);
        bool_from_env("RSFILC_CHARTS_PADDING", &mut self.padding);
    }

    fn check_app_config(&mut self) {
        // TODO somehow stop confy from destroying the width and height which it should have skipped
        let safe = (self.width, self.height);
        *self = CONFIG.charts.clone();
        (self.width, self.height) = safe;
    }
}

impl Default for ChartConfig {
    fn default() -> Self {
        let width = dimensions().map(|(w, _)| w as u32).unwrap_or(180); // NOTE magicnumber
        let aspect_ratio = (16, 9);
        let height = width * aspect_ratio.1 / aspect_ratio.0;

        Self {
            width,
            height,
            aspect_ratio,
            scale: 1.0f32,
            borders: true,
            padding: true,
            lines: false,
            use_color: true,
            plot_color: (241, 118, 52),
            bg_color: (118, 118, 118),
        }
    }
}
