import re

with open('crates/app_core/src/session.rs', 'r') as f:
    text = f.read()

def repl(m):
    return """
    fn canvas_dragged_batch(
        &mut self,
        points: &[(f32, f32)],
        current_state: &UiShellState,
    ) -> UiShellState {
        if points.is_empty() { return current_state.clone(); }
        let drag_started_at = Instant::now();
        
        let mapped_points: Vec<_> = points
            .iter()
            .map(|&(nx, ny)| {
                let (cx, cy, pt) = self.canvas_point(nx, ny);
                let current_screen = self.preview_screen_point(nx, ny);
                (nx, ny, cx, cy, pt, current_screen)
            })
            .collect();
            
        let (_, _, last_cx, last_cy, _, _) = *mapped_points.last().unwrap();
        
        let mut preview_changes: Option<Vec<PixelChange>> = None;
        let mut stroke_apply_metrics: Option<(f64, usize, usize)> = None;
        let active_stroke_tool = self.active_stroke_tool();
        
        let state = match self.canvas_interaction.as_mut() {
            Some(CanvasInteraction::Stroke(live_stroke)) => {
                let previous_sample = live_stroke.collector.samples().last().copied();
                let mut stroke_samples = Vec::with_capacity(mapped_points.len() + 1);
                if let Some(prev) = previous_sample {
                    stroke_samples.push(prev);
                }
                
                for &(_, _, _, _, pt, _) in &mapped_points {
                    let sample = StrokeSample::new(pt);
                    stroke_samples.push(sample);
                    live_stroke.collector.push_sample(sample);
                }

                if stroke_samples.len() > 1 {
                    let stroke_tool = active_stroke_tool
                        .expect("paint tools must expose stroke tools");
                    let apply_started_at = Instant::now();
                    let application =
                        stroke_tool.apply_stroke(&mut self.surface, &stroke_samples);
                    stroke_apply_metrics = Some((
                        elapsed_milliseconds(apply_started_at),
                        application.dab_count,
                        application.history_entry.changes().len(),
                    ));
                    preview_changes = Some(application.history_entry.changes().to_vec());
                    merge_pixel_changes(
                        &mut live_stroke.changes_by_pixel,
                        application.history_entry.changes(),
                    );
                    live_stroke.dab_count += application.dab_count;
                    self.document_dirty = true;
                }

                self.shell_state(format!(
                    "dragging {} at {}, {}",
                    self.active_tool_label().to_lowercase(),
                    last_cx,
                    last_cy,
                ))
            }
            Some(CanvasInteraction::Move { start }) => {
                let (_, _, _, _, pt, _) = *mapped_points.last().unwrap();
                let move_tool = MoveTool;
                let (delta_x, delta_y) = move_tool.drag_delta(*start, pt);
                self.shell_state(format!("move delta {} px, {} px", delta_x, delta_y))
            }
            Some(CanvasInteraction::Marquee { start }) => {
                self.document.select_rect(start.x as u32, start.y as u32, last_cx, last_cy);
                self.shell_state(format!("marquee selection to {}, {}", last_cx, last_cy))
            }
            Some(CanvasInteraction::Crop { start }) => {
                self.document.select_rect(start.x as u32, start.y as u32, last_cx, last_cy);
                self.shell_state(format!("crop region to {}, {}", last_cx, last_cy))
            }
            Some(CanvasInteraction::Hand { last_screen }) => {
                let (_, _, _, _, _, current_screen) = *mapped_points.last().unwrap();
                self.viewport.pan_by_screen_delta(Vector::new(
                    current_screen.x - last_screen.x,
                    current_screen.y - last_screen.y,
                ));
                *last_screen = current_screen;
                self.shell_state(format!("panning view at {}, {}", last_cx, last_cy))
            }
            None => self.shell_state(current_state.status_message.clone()),
        };

        let mut preview_update_ms = None;
        let mut preview_published = false;
        if let Some(changes) = preview_changes {
            if self.should_publish_interactive_preview() {
                let preview_started_at = Instant::now();
                self.update_preview_for_changes(&changes);
                self.last_interactive_preview_at.set(Some(Instant::now()));
                preview_update_ms = Some(elapsed_milliseconds(preview_started_at));
                preview_published = true;
            }
        }

        if let Some((apply_ms, dab_count, changed_pixels)) = stroke_apply_metrics {
            log_elapsed_ms(
                "brush_drag_segment",
                elapsed_milliseconds(drag_started_at),
                &[
                    ("apply_ms", format!("{apply_ms:.2}")),
                    (
                        "preview_ms",
                        format!("{:.2}", preview_update_ms.unwrap_or_default()),
                    ),
                    ("preview_published", preview_published.to_string()),
                    ("dabs", dab_count.to_string()),
                    ("changed_pixels", changed_pixels.to_string()),
                    ("cursor", format!("{last_cx},{last_cy}")),
                    ("batch_size", mapped_points.len().to_string()),
                ],
            );
        }

        state
    }
"""

text = re.sub(r'    fn canvas_dragged_batch\(\n        &mut self,\n        points: &\[\(f32, f32\)\],\n        current_state: &UiShellState,\n    \) -> UiShellState \{.*?(?=    fn canvas_released\()', repl, text, flags=re.DOTALL)

with open('crates/app_core/src/session.rs', 'w') as f:
    f.write(text)
