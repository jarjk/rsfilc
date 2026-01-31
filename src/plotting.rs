use rgb::RGB8;
use termize::dimensions;
use textplots::{Chart, ColorPlot, Plot, Shape};

/// High-level plotting helper
pub struct ChartBuilder {
    width: u32,
    height: u32,
    points: Vec<(f32, f32)>,
}

impl ChartBuilder {
    /// Create builder from already-prepared points
    pub fn new(points: Vec<(f32, f32)>) -> Self {
        let (chart_width, chart_height) = dimensions()
            .map(|(w, h)| (w as u32, h as u32))
            .unwrap_or((180, 60)); // magicnumber :D

        Self {
            points,
            width: chart_width,
            height: chart_height,
        }
    }

    /// Calculate and render the chart
    pub fn build_and_display(&mut self, borders: bool, ylines: bool, color: bool, padding: bool) {
        let (xmin, xmax, ymin, ymax) = self.infer_bounds(padding);

        let mut horizontal: Vec<f32> = Vec::new();
        let mut vertical: Vec<(f32, f32, f32)> = Vec::new();

        if borders {
            horizontal.push(ymax);
            horizontal.push(ymin);
            vertical.push((xmin, ymin, ymax));
            vertical.push((xmax, ymin, ymax));
        }
        if ylines {
            let start = ymin.ceil() as i32;
            let end = ymax.floor() as i32;
            for y in start..=end {
                horizontal.push(y as f32);
            }
        }

        let mut bg_line_buffers: Vec<Vec<(f32, f32)>> = Vec::new();
        for (x, y0, y1) in &vertical {
            bg_line_buffers.push(vec![(*x, *y0), (*x, *y1)]);
        }

        let mut bg_shapes: Vec<Shape> = Vec::new();
        for &y in &horizontal {
            bg_shapes.push(Shape::Continuous(Box::new(move |_| y)));
        }
        for line in &bg_line_buffers {
            bg_shapes.push(Shape::Lines(line));
        }

        let data = Shape::Lines(&self.points);
        let mut chart = Chart::new_with_y_range(self.width, self.height, xmin, xmax, ymin, ymax);

        // draw background shapes
        let chart = bg_shapes.iter().fold(&mut chart, |chart, shape| {
            if color {
                chart.linecolorplot(
                    shape,
                    RGB8 {
                        r: 118,
                        g: 118,
                        b: 118,
                    },
                )
            } else {
                chart.lineplot(shape)
            }
        });

        let chart = if color {
            chart.linecolorplot(
                &data,
                RGB8 {
                    r: 30,
                    g: 144,
                    b: 255,
                },
            )
        } else {
            chart.lineplot(&data)
        };

        chart.figures();
        print!("{chart}");
    }

    /// Infer bounds from points
    fn infer_bounds(&self, padding: bool) -> (f32, f32, f32, f32) {
        let xmin = 0f32;
        let xmax = self.points.len() as f32;
        let ymin = 1f32;
        let ymax = 5f32;

        if padding {
            let x_span = (xmax - xmin).max(1.0);
            let x_pad = x_span * 0.02;
            return (xmin - x_pad, xmax + x_pad, ymin, ymax);
        }

        (xmin, xmax, ymin, ymax)
    }
}
