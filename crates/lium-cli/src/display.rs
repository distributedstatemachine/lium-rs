use crate::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use lium_core::{ExecutorInfo, PodInfo, TemplateInfo};
use std::collections::HashMap;

/// Table formatting utilities
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    max_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<String>) -> Self {
        let max_widths = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            max_widths,
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        // Update max widths
        for (i, cell) in row.iter().enumerate() {
            if i < self.max_widths.len() {
                self.max_widths[i] = self.max_widths[i].max(cell.len());
            }
        }
        self.rows.push(row);
    }

    pub fn print(&self) {
        self.print_separator();
        self.print_header();
        self.print_separator();

        for row in &self.rows {
            self.print_row(row);
        }

        self.print_separator();
    }

    fn print_separator(&self) {
        print!("┌");
        for (i, &width) in self.max_widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < self.max_widths.len() - 1 {
                print!("┬");
            }
        }
        println!("┐");
    }

    fn print_header(&self) {
        print!("│");
        for (i, header) in self.headers.iter().enumerate() {
            print!(" {:<width$} ", header.bold(), width = self.max_widths[i]);
            print!("│");
        }
        println!();
    }

    fn print_row(&self, row: &[String]) {
        print!("│");
        for (i, cell) in row.iter().enumerate() {
            let width = if i < self.max_widths.len() {
                self.max_widths[i]
            } else {
                0
            };
            print!(" {:<width$} ", cell, width = width);
            print!("│");
        }
        println!();
    }
}

/// Display executors in a formatted table
pub fn display_executors_table(executors: &[ExecutorInfo], show_pareto: bool) {
    if executors.is_empty() {
        println!("{}", "No executors found.".yellow());
        return;
    }

    let mut table = Table::new(vec![
        "Index".to_string(),
        "HUID".to_string(),
        "GPU Type".to_string(),
        "Count".to_string(),
        "$/GPU/hr".to_string(),
        "$/hr".to_string(),
        "RAM (GB)".to_string(),
        "Location".to_string(),
        "Status".to_string(),
    ]);

    for (i, executor) in executors.iter().enumerate() {
        let index = (i + 1).to_string();
        let gpu_count = executor.gpu_count.to_string();
        let price_gpu = format!("{:.3}", executor.price_per_gpu_hour);
        let price_total = format!("{:.3}", executor.price_per_hour);

        // Extract RAM from specs
        let ram = executor
            .specs
            .get("memory_gb")
            .and_then(|v| v.as_f64())
            .map(|v| format!("{:.0}", v))
            .unwrap_or_else(|| "N/A".to_string());

        // Extract location
        let location = executor
            .location
            .get("region")
            .or_else(|| executor.location.get("country"))
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        // Status with color
        let status = if executor.available {
            "Available".green().to_string()
        } else {
            "Rented".red().to_string()
        };

        table.add_row(vec![
            index,
            executor.huid.clone(),
            executor.gpu_type.clone(),
            gpu_count,
            price_gpu,
            price_total,
            ram,
            location,
            status,
        ]);
    }

    table.print();

    if show_pareto {
        println!(
            "\n{}",
            "✓ Showing Pareto optimal executors (best price/performance)".green()
        );
    }
}

/// Display GPU type summary
pub fn display_gpu_summary(gpu_types: &HashMap<String, Vec<ExecutorInfo>>) {
    if gpu_types.is_empty() {
        println!("{}", "No GPU types found.".yellow());
        return;
    }

    let mut table = Table::new(vec![
        "GPU Type".to_string(),
        "Count".to_string(),
        "Min $/GPU/hr".to_string(),
        "Max $/GPU/hr".to_string(),
        "Available".to_string(),
    ]);

    for (gpu_type, executors) in gpu_types {
        let count = executors.len().to_string();
        let available_count = executors.iter().filter(|e| e.available).count();

        let prices: Vec<f64> = executors.iter().map(|e| e.price_per_gpu_hour).collect();
        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        table.add_row(vec![
            gpu_type.clone(),
            count,
            format!("{:.3}", min_price),
            format!("{:.3}", max_price),
            format!("{}/{}", available_count, executors.len()),
        ]);
    }

    table.print();
}

/// Display pods in a formatted table
pub fn display_pods_table(pods: &[PodInfo]) {
    if pods.is_empty() {
        println!("{}", "No active pods found.".yellow());
        return;
    }

    let mut table = Table::new(vec![
        "Index".to_string(),
        "Pod HUID".to_string(),
        "Status".to_string(),
        "GPU Config".to_string(),
        "Uptime".to_string(),
        "SSH Command".to_string(),
    ]);

    for (i, pod) in pods.iter().enumerate() {
        let index = (i + 1).to_string();

        // Extract GPU info from executor
        let gpu_config = pod
            .executor
            .get("gpu_type")
            .and_then(|v| v.as_str())
            .map(|gpu| {
                let count = pod
                    .executor
                    .get("gpu_count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                format!("{}x {}", count, gpu)
            })
            .unwrap_or_else(|| "Unknown".to_string());

        // Calculate uptime
        let uptime = if let Some(created_at) = pod.created_at {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(created_at);

            let days = duration.num_days();
            let hours = duration.num_hours() % 24;
            let minutes = duration.num_minutes() % 60;

            if days > 0 {
                format!("{}d {}h {}m", days, hours, minutes)
            } else if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            }
        } else {
            "Unknown".to_string()
        };

        // Status with color
        let status = match pod.status.as_str() {
            "running" => pod.status.green().to_string(),
            "starting" => pod.status.yellow().to_string(),
            "stopped" => pod.status.red().to_string(),
            _ => pod.status.to_string(),
        };

        // SSH command (truncated if too long)
        let ssh_cmd = pod
            .ssh_cmd
            .as_ref()
            .map(|cmd| {
                if cmd.len() > 40 {
                    format!("{}...", &cmd[..37])
                } else {
                    cmd.clone()
                }
            })
            .unwrap_or_else(|| "N/A".to_string());

        table.add_row(vec![
            index,
            pod.huid.clone(),
            status,
            gpu_config,
            uptime,
            ssh_cmd,
        ]);
    }

    table.print();
}

/// Display detailed pod information
pub fn display_pod_details(pod: &PodInfo) {
    println!("{}", format!("Pod Details: {}", pod.name).bold().blue());
    println!("  {}: {}", "HUID".bold(), pod.huid);
    println!("  {}: {}", "Status".bold(), pod.status);
    println!("  {}: {}", "ID".bold(), pod.id);

    if let Some(ssh_cmd) = &pod.ssh_cmd {
        println!("  {}: {}", "SSH Command".bold(), ssh_cmd.green());
    }

    if !pod.ports.is_empty() {
        println!("  {}:", "Port Mappings".bold());
        for (service, port) in &pod.ports {
            println!("    {}: {}", service, port);
        }
    }

    if let Some(created_at) = pod.created_at {
        println!(
            "  {}: {}",
            "Created".bold(),
            created_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    if let Some(updated_at) = pod.updated_at {
        println!(
            "  {}: {}",
            "Updated".bold(),
            updated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    println!();
}

/// Display templates in a formatted table
pub fn display_templates_table(templates: &[TemplateInfo]) {
    if templates.is_empty() {
        println!("{}", "No templates found.".yellow());
        return;
    }

    let mut table = Table::new(vec![
        "Index".to_string(),
        "ID".to_string(),
        "Name".to_string(),
        "Docker Image".to_string(),
        "Status".to_string(),
    ]);

    for (i, template) in templates.iter().enumerate() {
        let index = (i + 1).to_string();
        let image = if let Some(tag) = &template.docker_image_tag {
            format!("{}:{}", template.docker_image, tag)
        } else {
            template.docker_image.clone()
        };

        let status = template
            .status
            .as_ref()
            .map(|s| match s.as_str() {
                "VERIFY_SUCCESS" => s.green().to_string(),
                "VERIFY_PENDING" => s.yellow().to_string(),
                "VERIFY_FAILED" => s.red().to_string(),
                _ => s.to_string(),
            })
            .unwrap_or_else(|| "Unknown".to_string());

        table.add_row(vec![
            index,
            template.id.clone(),
            template.name.clone(),
            image,
            status,
        ]);
    }

    table.print();
}

/// Interactive prompts
pub fn prompt_confirm(message: &str, default: bool) -> Result<bool> {
    let result = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .default(default)
        .interact()
        .map_err(|e| crate::CliError::OperationFailed(e.to_string()))?;

    Ok(result)
}

pub fn prompt_select<T: ToString>(message: &str, items: &[T]) -> Result<usize> {
    let item_strings: Vec<String> = items.iter().map(|item| item.to_string()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .items(&item_strings)
        .default(0)
        .interact()
        .map_err(|e| crate::CliError::OperationFailed(e.to_string()))?;

    Ok(selection)
}

pub fn prompt_input(message: &str, default: Option<&str>) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut input = Input::with_theme(&theme).with_prompt(message);

    if let Some(default_val) = default {
        input = input.default(default_val.to_string());
    }

    let result = input
        .interact_text()
        .map_err(|e| crate::CliError::OperationFailed(e.to_string()))?;

    Ok(result)
}

/// Status messages
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

pub fn print_error(message: &str) {
    println!("{} {}", "✗".red().bold(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Progress indicators
pub fn print_spinner_start(message: &str) {
    print!("{} {}...", "⠋".blue().bold(), message);
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}

pub fn print_spinner_stop() {
    println!(" {}", "Done".green());
}

// TODO: Add progress bars for long operations
// TODO: Add more sophisticated table formatting options
// TODO: Add theme support for different color schemes
// TODO: Add export options (JSON, CSV)
// TODO: Add pagination for large tables

pub fn placeholder() {}
