use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
    pub(crate) fn robot(&mut self, ui: &mut egui::Ui) {
        if let Some(answer) = self.poll_robot_job() {
            self.robot_output = answer.clone();
            self.push_chat_message("ROBOTCYB", answer, false);
            self.add_event("ROBOT", "RobotCYB worker completed the request");
        }

        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(55, 145, 105);
        let panel = Color32::from_rgb(5, 18, 13);

        ui.vertical(|ui| {
            ui.label(RichText::new("◉  ROBOTCYB").size(24.0).strong().color(neon));

            ui.label(
                RichText::new("LOCAL AI AGENT · REQUEST → REASONING → RESPONSE")
                    .size(11.0)
                    .color(dim),
            );

            ui.add_space(14.0);

            // ------------------------------------------------
            // ROBOT CORE / EYE
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(14))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let available = ui.available_width();

                        let eye_size = 150.0;

                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(eye_size, eye_size),
                            egui::Sense::hover(),
                        );

                        let painter = ui.painter_at(rect);
                        let c = rect.center();

                        painter.circle_stroke(c, 62.0, Stroke::new(2.0, neon));

                        painter.circle_stroke(c, 48.0, Stroke::new(1.0, dim));

                        painter.circle_filled(
                            c,
                            27.0,
                            Color32::from_rgba_unmultiplied(0, 255, 150, 28),
                        );

                        painter.circle_stroke(c, 27.0, Stroke::new(2.0, neon));

                        painter.circle_filled(c, 10.0, neon);

                        painter.circle_filled(c + Vec2::new(-4.0, -5.0), 3.0, Color32::WHITE);

                        // eye rays
                        for k in 0..8 {
                            let a = k as f32 * std::f32::consts::TAU / 8.0;
                            let a0 = c + Vec2::angled(a) * 69.0;
                            let a1 = c + Vec2::angled(a) * 78.0;

                            painter.line_segment([a0, a1], Stroke::new(1.0, dim));
                        }

                        ui.add_space(18.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new("ROBOTCYB").size(22.0).strong().color(neon));

                            ui.label(RichText::new("LOCAL AGENT").size(11.0).strong().color(dim));

                            ui.add_space(8.0);

                            ui.label(RichText::new(format!("• {}", self.robot_status)).size(11.0).color(neon));

                            ui.label(
                                RichText::new(format!("QWEN · {}", self.qwen_status))
                                    .size(10.0)
                                    .color(dim),
                            );

                            ui.add_space(10.0);

                            ui.label(RichText::new("REQUEST").size(10.0).strong().color(dim));

                            ui.label(
                                RichText::new(if self.robot_job.is_some() { "RobotCYB is processing the request..." } else { "Ask the local RobotCYB agent anything." })
                                    .size(11.0)
                                    .color(Color32::from_rgb(125, 180, 150)),
                            );

                            let _ = available;
                        });
                    });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // REQUEST
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("REQUEST").size(11.0).strong().color(neon));

                    ui.add_space(5.0);

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.robot_input)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter a request for RobotCYB..."),
                    );

                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        let busy = self.robot_job.is_some();
                        let send = ui
                            .add_enabled(
                                !busy,
                                egui::Button::new(
                                    RichText::new("◉  SEND REQUEST")
                                        .size(12.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(170.0, 34.0)),
                            )
                            .clicked();

                        if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || send
                        {
                            let q = self.robot_input.trim().to_string();

                            if !q.is_empty() && !busy {
                                self.push_chat_message("YOU", q.clone(), true);
                                self.robot_input.clear();
                                self.robot_output = "ROBOTCYB · PROCESSING\n\nRequest accepted by bounded worker.".into();
                                self.add_event("ROBOT", &format!("RobotCYB accepted request: {}", q));
                                let _ = self.start_robot_job(q);
                            }
                        }

                        ui.label(RichText::new(if busy { "PROCESSING" } else { "ENTER · SEND" }).size(9.0).color(dim));
                    });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // RESPONSE
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("RESPONSE").size(11.0).strong().color(neon));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("LOCAL OUTPUT").size(9.0).color(dim));
                        });
                    });

                    ui.add_space(6.0);

                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 90))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_min_height(145.0);

                            ui.label(
                                RichText::new(&self.robot_output)
                                    .size(12.0)
                                    .color(Color32::from_rgb(175, 235, 205)),
                            );
                        });


            // ------------------------------------------------
            // AGENT WORK QUEUE · LOCAL, AUDITABLE TASK CONTRACTS
            // ------------------------------------------------
            ui.add_space(14.0);
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("AGENT WORK QUEUE").size(13.0).strong().color(neon));
                    ui.label(
                        RichText::new("Define verifiable work before execution. Budgets are limits, not payment authorizations.")
                            .size(10.0)
                            .color(dim),
                    );
                    ui.add_space(8.0);

                    let mut title = self.store.get("agent_work_form_title").unwrap_or_default();
                    let mut description = self.store.get("agent_work_form_description").unwrap_or_default();
                    let mut criteria = self.store.get("agent_work_form_criteria").unwrap_or_default();
                    let mut budget = self.store.get("agent_work_form_budget").unwrap_or_else(|| "5".into());
                    ui.label(RichText::new("TASK TITLE").size(9.0).strong().color(dim));
                    ui.text_edit_singleline(&mut title);
                    ui.label(RichText::new("DESCRIPTION").size(9.0).strong().color(dim));
                    ui.add(egui::TextEdit::multiline(&mut description).desired_rows(2).hint_text("What useful outcome should the agent produce?"));
                    ui.label(RichText::new("ACCEPTANCE CRITERIA").size(9.0).strong().color(dim));
                    ui.add(egui::TextEdit::multiline(&mut criteria).desired_rows(2).hint_text("How will a human verify the result?"));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("BUDGET CEILING").size(9.0).strong().color(dim));
                        ui.add(egui::TextEdit::singleline(&mut budget).desired_width(90.0));
                        ui.label(RichText::new("accounting units · no funds moved").size(9.0).color(dim));
                    });
                    self.store.set("agent_work_form_title", &title);
                    self.store.set("agent_work_form_description", &description);
                    self.store.set("agent_work_form_criteria", &criteria);
                    self.store.set("agent_work_form_budget", &budget);

                    if ui.button(RichText::new("＋ CREATE PROPOSAL").strong().color(neon)).clicked() {
                        let parsed_budget = budget.trim().parse::<f64>();
                        let result = parsed_budget
                            .map_err(|_| "Budget must be a valid number".to_string())
                            .and_then(|limit| crate::agent_economy::AgentTask::proposal(
                                title.clone(), description.clone(), criteria.clone(), limit, 0.0
                            ));
                        match result {
                            Ok(task) => match self.store.save_agent_task(&task) {
                                Ok(()) => {
                                    self.store.set("agent_work_form_title", "");
                                    self.store.set("agent_work_form_description", "");
                                    self.store.set("agent_work_form_criteria", "");
                                    self.add_event("AGENT_WORK", &format!("Created task proposal: {}", task.title));
                                    self.notify("AGENT TASK PROPOSAL SAVED");
                                }
                                Err(error) => self.notify(format!("TASK SAVE FAILED: {error}")),
                            },
                            Err(error) => self.notify(format!("TASK NOT CREATED: {error}")),
                        }
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("PERSISTED TASKS").size(10.0).strong().color(neon));
                    let tasks = self.store.agent_tasks();
                    if tasks.is_empty() {
                        ui.label(RichText::new("No tasks yet. Create a proposal above.").size(10.0).color(dim));
                    }
                    let mut update: Option<crate::agent_economy::AgentTask> = None;
                    for task in tasks.iter().take(20) {
                        ui.separator();
                        ui.label(RichText::new(&task.title).strong().color(Color32::from_rgb(180, 235, 205)));
                        ui.label(RichText::new(format!(
                            "{} · ceiling {:.2} · estimate {:.2}",
                            task.status.as_str(), task.budget_limit, task.estimated_cost
                        )).size(9.0).color(dim));
                        ui.label(RichText::new(&task.acceptance_criteria).size(10.0).color(Color32::from_rgb(145, 190, 165)));
                        if ui.small_button("＋ NEW WORKFLOW RUN").clicked() {
                            let input = serde_json::json!({
                                "task_id": task.id,
                                "title": task.title,
                                "description": task.description,
                                "acceptance_criteria": task.acceptance_criteria
                            });
                            match crate::agent_runtime::WorkflowRun::new("robotcyb_task_workflow", Some(task.id.clone()), input) {
                                Ok(run) => match self.store.save_workflow_run(&run) {
                                    Ok(()) => {
                                        self.add_event("AGENT_WORKFLOW", &format!("Created workflow run for '{}'", task.title));
                                        self.notify("WORKFLOW RUN SAVED");
                                    }
                                    Err(error) => self.notify(format!("WORKFLOW SAVE FAILED: {error}")),
                                },
                                Err(error) => self.notify(format!("WORKFLOW NOT CREATED: {error}")),
                            }
                        }
                        ui.horizontal_wrapped(|ui| {
                            use crate::agent_economy::AgentTaskStatus as Status;
                            let next = match &task.status {
                                Status::Proposed => Some((Status::Ready, "MARK READY")),
                                Status::Ready => Some((Status::InProgress, "START")),
                                Status::InProgress => Some((Status::Submitted, "SUBMIT")),
                                Status::Submitted => Some((Status::Accepted, "ACCEPT")),
                                Status::Rejected => Some((Status::InProgress, "RETRY")),
                                _ => None,
                            };
                            if let Some((next_status, label)) = next {
                                if ui.small_button(label).clicked() {
                                    let mut changed = task.clone();
                                    if changed.transition(next_status).is_ok() {
                                        update = Some(changed);
                                    }
                                }
                            }
                            if matches!(&task.status, Status::Proposed | Status::Ready | Status::InProgress | Status::Rejected)
                                && ui.small_button("CANCEL").clicked()
                            {
                                let mut changed = task.clone();
                                if changed.transition(Status::Cancelled).is_ok() {
                                    update = Some(changed);
                                }
                            }
                        });
                    }
                    if let Some(task) = update {
                        match self.store.save_agent_task(&task) {
                            Ok(()) => {
                                self.add_event("AGENT_WORK", &format!("Task '{}' moved to {}", task.title, task.status.as_str()));
                                self.notify(format!("TASK STATUS: {}", task.status.as_str().to_uppercase()));
                            }
                            Err(error) => self.notify(format!("TASK UPDATE FAILED: {error}")),
                        }
                    }

                    ui.add_space(14.0);
                    ui.label(RichText::new("WORKFLOW RUN TIMELINE").size(11.0).strong().color(neon));
                    ui.label(RichText::new("Runs are persisted and advanced manually. No arbitrary tools or external side effects are executed.").size(9.0).color(dim));
                    let runs = self.store.workflow_runs();
                    if runs.is_empty() {
                        ui.label(RichText::new("Create a run from any persisted task above.").size(10.0).color(dim));
                    }
                    let mut run_update: Option<crate::agent_runtime::WorkflowRun> = None;
                    let mut new_checkpoint: Option<crate::agent_runtime::WorkflowCheckpoint> = None;
                    for run in runs.iter().take(15) {
                        ui.separator();
                        ui.label(RichText::new(format!("{} · {}", run.workflow_name, run.id.chars().take(8).collect::<String>())).strong().color(Color32::from_rgb(180, 235, 205)));
                        ui.label(RichText::new(format!("{} · step {} · updated {}", run.status.as_str(), run.step, run.updated_at)).size(9.0).color(dim));
                        ui.horizontal_wrapped(|ui| {
                            if matches!(&run.status, crate::agent_runtime::WorkflowStatus::Running) {
                                if ui.small_button("RUN CONTRACT CHECK").clicked() {
                                    let mut registry = crate::agent_runtime::CapabilityRegistry::default();
                                    let registration = crate::agent_runtime::CapabilitySpec::new(
                                        "task.validate_contract",
                                        "Check required task fields before execution",
                                        crate::agent_runtime::CapabilityRisk::ReadOnly,
                                        false,
                                        crate::agent_runtime::MAX_INPUT_BYTES,
                                    ).and_then(|spec| registry.register(
                                        spec,
                                        std::sync::Arc::new(crate::agent_runtime::TaskContractValidator),
                                    ));
                                    if let Err(error) = registration {
                                        self.notify(format!("VALIDATOR SETUP FAILED: {error}"));
                                    } else {
                                        match crate::agent_runtime::WorkflowRunner::new(&registry, 8) {
                                        Ok(runner) => {
                                            let plan = crate::agent_runtime::WorkflowPlan {
                                                name: "task_contract_validation".into(),
                                                steps: vec![crate::agent_runtime::WorkflowStep {
                                                    capability_id: "task.validate_contract".into(),
                                                    input: run.input.clone(),
                                                    approved: false,
                                                }],
                                            };
                                            match runner.run(&plan) {
                                                Ok(report) => {
                                                    let report_json = serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({"error":"report serialization failed"}));
                                                    let mut changed = run.clone();
                                                    if changed.checkpoint(report_json.clone()).is_ok() {
                                                        match crate::agent_runtime::WorkflowCheckpoint::new(
                                                            run.id.clone(), changed.step, "contract_validation",
                                                            "Deterministic local task-contract validation", report_json
                                                        ) {
                                                            Ok(checkpoint) => {
                                                                run_update = Some(changed);
                                                                new_checkpoint = Some(checkpoint);
                                                            }
                                                            Err(error) => self.notify(format!("VALIDATION CHECKPOINT FAILED: {error}")),
                                                        }
                                                    } else {
                                                        self.notify("CONTRACT CHECK COULD NOT BE RECORDED");
                                                    }
                                                }
                                                Err(error) => self.notify(format!("CONTRACT CHECK FAILED: {error}")),
                                            }
                                        }
                                        Err(error) => self.notify(format!("VALIDATOR SETUP FAILED: {error}")),
                                        }
                                    }
                                }
                            }
                            use crate::agent_runtime::WorkflowStatus as Status;
                            let next = match &run.status {
                                Status::Planned => Some((Status::Running, "START RUN")),
                                Status::Running => Some((Status::Validating, "BEGIN VALIDATION")),
                                Status::Checkpointed => Some((Status::Running, "RESUME")),
                                Status::Validating => Some((Status::Submitted, "SUBMIT RESULT")),
                                Status::Submitted => Some((Status::Accepted, "ACCEPT RESULT")),
                                Status::Rejected => Some((Status::Running, "RETRY RUN")),
                                _ => None,
                            };
                            if let Some((status, label)) = next {
                                if ui.small_button(label).clicked() {
                                    let mut changed = run.clone();
                                    if changed.transition(status).is_ok() {
                                        run_update = Some(changed);
                                    }
                                }
                            }
                            if matches!(&run.status, Status::Running | Status::Checkpointed) && ui.small_button("CHECKPOINT").clicked() {
                                let mut changed = run.clone();
                                let data = serde_json::json!({
                                    "step_before_checkpoint": run.step,
                                    "note": "Manual checkpoint from RobotCYB UI; no tool was executed."
                                });
                                if changed.checkpoint(data.clone()).is_ok() {
                                    let checkpoint = crate::agent_runtime::WorkflowCheckpoint::new(
                                        run.id.clone(), changed.step, "manual_checkpoint",
                                        "User-triggered checkpoint", data
                                    );
                                    match checkpoint {
                                        Ok(checkpoint) => {
                                            run_update = Some(changed);
                                            new_checkpoint = Some(checkpoint);
                                        }
                                        Err(error) => self.notify(format!("CHECKPOINT REJECTED: {error}")),
                                    }
                                }
                            }
                            if matches!(&run.status, Status::Planned | Status::Running | Status::Checkpointed | Status::Validating | Status::Rejected)
                                && ui.small_button("CANCEL RUN").clicked()
                            {
                                let mut changed = run.clone();
                                if changed.transition(Status::Cancelled).is_ok() {
                                    run_update = Some(changed);
                                }
                            }
                        });
                        let checkpoints = self.store.workflow_checkpoints(&run.id);
                        ui.label(RichText::new(format!("{} checkpoint(s)", checkpoints.len())).size(9.0).color(dim));
                    }
                    if let Some(run) = run_update {
                        match self.store.save_workflow_run(&run) {
                            Ok(()) => {
                                if let Some(checkpoint) = new_checkpoint {
                                    if let Err(error) = self.store.append_workflow_checkpoint(&checkpoint) {
                                        self.notify(format!("CHECKPOINT LOG FAILED: {error}"));
                                    }
                                }
                                self.add_event("AGENT_WORKFLOW", &format!("Workflow {} moved to {}", &run.id[..8.min(run.id.len())], run.status.as_str()));
                                self.notify(format!("WORKFLOW STATUS: {}", run.status.as_str().to_uppercase()));
                            }
                            Err(error) => self.notify(format!("WORKFLOW UPDATE FAILED: {error}")),
                        }
                    }
                });
                });
        });
    }
}
