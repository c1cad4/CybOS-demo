//! Agent execution loop and tool orchestration.

use crate::CybOs;
use super::web_planner;

impl CybOs {
    pub(crate) fn run_agent_loop(&mut self, q: &str, conversation: String) -> String {
        // Normal local agent path
        // ----------------------------------------------------

        let mut current_query = conversation.clone();
        let mut forced_tool: Option<(String, String)> = None;

        for step in 0..5 {
            let selection = if let Some(tool) = forced_tool.take() {
                Some(tool)
            } else {
                self.select_tool_with_qwen(&current_query)
            };

            let Some((tool, arguments)) = selection else {
                return self.robot_answer(q);
            };

            let result = match tool.as_str() {
                "system_status" => self.tool_system_status(),

                "get_events" => {
                    let limit = arguments.parse::<usize>().unwrap_or(10).clamp(1, 50);
                    self.tool_get_events(limit)
                }

                "get_entity" => self.tool_get_entity(&arguments),
                "search_knowledge" => self.tool_search_knowledge(&arguments),
                "farm_status" => self.tool_farm_status(),

                "web_search" | "web_fetch" => {
                    match self.handle_web_tool(
                        &tool,
                        &arguments,
                        &conversation,
                    ) {
                        web_planner::WebToolOutcome::Answer(answer) => {
                            return answer;
                        }
                        web_planner::WebToolOutcome::Continue(result) => result,
                    }
                }

                _ => {
                    return format!("RobotCYB: неизвестный инструмент {}.", tool);
                }
            };

            let observation = format!(
                "ORIGINAL CONVERSATION:

{}

AGENT STEP:
{}

LAST TOOL:
{}

TOOL ARGUMENTS:
{}

TOOL RESULT:
{}
",
                conversation,
                step + 1,
                tool,
                arguments,
                result
            );

            let Some(decision) = self.planner_decision_from_observation(&observation) else {
                return "RobotCYB не смог завершить запрос в проверяемом режиме.".into();
            };

            match decision["action"].as_str() {
                Some("final") => {
                    if let Some(answer) = decision["answer"].as_str() {
                        if !answer.trim().is_empty() {
                            return answer.to_string();
                        }
                    }

                    return self.robot_answer(q);
                }

                Some("tool") => {
                    let next_tool = decision["tool"].as_str().unwrap_or("");

                    let next_arguments = decision["arguments"].as_str().unwrap_or("");

                    if next_tool.is_empty() {
                        return self.robot_answer(q);
                    }

                    forced_tool = Some((next_tool.to_string(), next_arguments.to_string()));

                    current_query = format!(
                        "{}\n\nPrevious tool result:\n{}\n\nRequested next tool: {}\nArguments: {}",
                        conversation, result, next_tool, next_arguments
                    );
                }

                _ => {
                    return self.robot_answer(q);
                }
            }
        }

        "RobotCYB достиг максимального числа шагов агента.".into()
    
    }

    pub(crate) fn start_robot_job(&mut self, query: String) -> bool {
        if self.robot_job.is_some() {
            return false;
        }

        let snapshot = AgentSnapshot {
            node_id: self.node_id.clone(),
            status: self.status.clone(),
            qwen_status: self.qwen_status.clone(),
            temperature: self.temperature,
            battery: self.battery,
            chat: self.chat.clone(),
            events: self.events.clone(),
            nodes: self.nodes.clone(),
            links: self.links.clone(),
        };

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut worker = match CybOs::from_agent_snapshot(snapshot) {
                Ok(worker) => worker,
                Err(error) => {
                    let _ = tx.send(Err(error));
                    return;
                }
            };

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                worker.agent_answer(&query)
            }))
            .map_err(|_| "RobotCYB worker terminated unexpectedly.".to_string());

            let _ = tx.send(result);
        });

        self.robot_job = Some(rx);
        self.robot_status = "RUNNING".into();
        self.runtime.set_status("ROBOTCYB", "RUNNING");
        true
    }

    pub(crate) fn poll_robot_job(&mut self) -> Option<String> {
        let Some(receiver) = self.robot_job.as_ref() else {
            return None;
        };

        match receiver.try_recv() {
            Ok(Ok(answer)) => {
                self.robot_job = None;
                self.robot_status = "READY".into();
                self.runtime.set_status("ROBOTCYB", "READY");
                Some(answer)
            }
            Ok(Err(error)) => {
                self.robot_job = None;
                self.robot_status = "ERROR".into();
                self.runtime.set_status("ROBOTCYB", "ERROR");
                Some(format!("RobotCYB worker error: {}", error))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.robot_status = "RUNNING".into();
                self.runtime.set_status("ROBOTCYB", "RUNNING");
                None
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.robot_job = None;
                self.robot_status = "ERROR".into();
                self.runtime.set_status("ROBOTCYB", "ERROR");
                Some("RobotCYB worker disconnected before returning a result.".into())
            }
        }
    }

    fn from_agent_snapshot(snapshot: AgentSnapshot) -> Result<Self, String> {
        let store = crate::runtime::open_store();
        let (_lan_tx, lan_events) = std::sync::mpsc::channel();

        Ok(Self {
            store,
            page: crate::navigation::Page::Robot,
            search: String::new(),
            robot_input: String::new(),
            robot_output: String::new(),
            robot_job: None,
            robot_status: "WORKER".into(),
            chat_input: String::new(),
            chat_output: String::new(),
            chat: snapshot.chat,
            cicada_wallet: crate::config::DEFAULT_CICADA_WALLET.into(),
            cicada_balance: None,
            sol_balance: None,
            payment_uri: String::new(),
            payment_status: String::new(),
            balance_refresh: std::time::Instant::now(),
            balance_task: None,
            events: snapshot.events,
            nodes: snapshot.nodes,
            links: snapshot.links,
            graph_zoom: 1.0,
            graph_pan: eframe::egui::Vec2::ZERO,
            selected_node: None,
            temperature: snapshot.temperature,
            battery: snapshot.battery,
            node_id: snapshot.node_id,
            runtime: crate::runtime::Runtime::new(),
            status: snapshot.status,
            qwen_child: None,
            qwen_status: snapshot.qwen_status,
            qwen_retry_after: std::time::Instant::now(),
            toast: None,
            camera_zone: 0,
            search_focus: false,
            last_scan: None,
            lan_peers: Vec::new(),
            lan_scan: None,
            lan_events,
            lan_send_task: None,
            lan_target: None,
            lan_delivery_status: String::new(),
            radar_visible: false,
            radar_visibility: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ble_advertiser: None,
            ble_advertiser_retry_after: std::time::Instant::now(),
            ble_peers: Vec::new(),
            ble_scan: None,
            ble_status: "BLE · WORKER DISABLED".into(),
            nearby_peers: Vec::new(),
            noise_private_key: Vec::new(),
            secure_events: {
                let (_tx, rx) = std::sync::mpsc::channel();
                rx
            },
            secure_send_task: None,
            secure_status: "SECURE CHAT · WORKER CONTEXT".into(),
            remember_note: String::new(),
        })
    }
}

struct AgentSnapshot {
    node_id: String,
    status: String,
    qwen_status: String,
    temperature: f32,
    battery: f32,
    chat: Vec<(String, String, bool)>,
    events: Vec<crate::models::Event>,
    nodes: Vec<crate::models::GraphNode>,
    links: Vec<crate::models::GraphLink>,
}
