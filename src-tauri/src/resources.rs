use serde::Serialize;

// Server-Ressourcen-Monitoring: reads `ps` output over the existing SSH
// connection rather than assuming a Linux /proc filesystem - this dev server
// is FreeBSD (see [[m2manager-db-schema]] memory), where /proc isn't
// mounted by default. `ps -axo pid,pcpu,pmem,rss,comm` is supported by both
// FreeBSD's and Linux's `ps`, so the same command/parser works on either.
//
// Which process names actually belong to the game server was never verified
// against a live server (unlike e.g. item_proto's schema, which was checked
// via information_schema) - so the name list stays a user-editable setting
// with a plausible default ("game,db", the common Metin2 core process
// names) instead of being hardcoded.

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessUsage {
    pub pid: u32,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub rss_kb: u64,
    pub command: String,
}

/// Parses `ps -axo pid,pcpu,pmem,rss,comm` output, keeping only rows whose
/// command contains one of `process_names` (case-insensitive substring). An
/// empty `process_names` list disables the filter (every process is kept) -
/// used to let the user see the raw listing while figuring out which names
/// to configure.
pub fn parse_and_filter(output: &str, process_names: &[String]) -> Vec<ProcessUsage> {
    let names_lower: Vec<String> = process_names.iter().map(|n| n.to_lowercase()).collect();
    output
        .lines()
        .skip(1) // header row (PID %CPU %MEM RSS COMMAND)
        .filter_map(|line| {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() < 5 {
                return None;
            }
            let pid: u32 = tokens[0].parse().ok()?;
            let cpu_percent: f64 = tokens[1].parse().ok()?;
            let mem_percent: f64 = tokens[2].parse().ok()?;
            let rss_kb: u64 = tokens[3].parse().ok()?;
            let command = tokens[4..].join(" ");
            if !names_lower.is_empty()
                && !names_lower.iter().any(|n| command.to_lowercase().contains(n))
            {
                return None;
            }
            Some(ProcessUsage {
                pid,
                cpu_percent,
                mem_percent,
                rss_kb,
                command,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    /// Only truly free pages (`vm.stats.vm.v_free_count`), not reclaimable
    /// cache/inactive memory - deliberately conservative rather than
    /// guessing which additional VM stat categories should count as "free"
    /// on this FreeBSD version, so it can read lower than `top`'s summary.
    pub free_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiskInfo {
    pub total_kb: u64,
    pub used_kb: u64,
    pub avail_kb: u64,
    pub capacity_percent: u8,
    pub mount_point: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ServerSystemInfo {
    pub memory: Option<MemoryInfo>,
    pub disk: Option<DiskInfo>,
}

const DISK_MARKER: &str = "---M2MANAGER-DISK---";

/// Builds the combined remote command for memory + disk info - both in one
/// SSH round trip (`run_command_streaming` opens a fresh SSH connection per
/// call, so bundling avoids paying that twice). `hw.physmem`/`hw.pagesize`/
/// `vm.stats.vm.v_free_count` are standard, stable FreeBSD sysctl names;
/// `df -Pk` forces single-line POSIX output so long filesystem device names
/// can't wrap onto a second line the way default `df` output sometimes does.
pub fn build_system_info_command(disk_path: &str) -> String {
    format!(
        "sysctl -n hw.physmem hw.pagesize vm.stats.vm.v_free_count; echo '{DISK_MARKER}'; df -Pk {}",
        crate::db_backup::shell_single_quote(disk_path)
    )
}

fn parse_memory_sysctl(output: &str) -> Option<MemoryInfo> {
    let nums: Vec<u64> = output.lines().filter_map(|l| l.trim().parse::<u64>().ok()).collect();
    if nums.len() < 3 {
        return None;
    }
    let (physmem, pagesize, free_count) = (nums[0], nums[1], nums[2]);
    Some(MemoryInfo {
        total_bytes: physmem,
        free_bytes: free_count * pagesize,
    })
}

fn parse_disk_usage(output: &str) -> Option<DiskInfo> {
    // Skip the header line ("Filesystem 1024-blocks Used Available Capacity
    // Mounted on") by only accepting lines that actually parse as numeric in
    // the expected positions, rather than assuming a fixed header line count.
    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 6 {
            continue;
        }
        let (Ok(total_kb), Ok(used_kb), Ok(avail_kb)) = (
            tokens[1].parse::<u64>(),
            tokens[2].parse::<u64>(),
            tokens[3].parse::<u64>(),
        ) else {
            continue;
        };
        let capacity_percent = tokens[4].trim_end_matches('%').parse::<u8>().unwrap_or(0);
        let mount_point = tokens[5..].join(" ");
        return Some(DiskInfo {
            total_kb,
            used_kb,
            avail_kb,
            capacity_percent,
            mount_point,
        });
    }
    None
}

pub fn parse_system_info_output(output: &str) -> ServerSystemInfo {
    let mut parts = output.splitn(2, DISK_MARKER);
    let mem_part = parts.next().unwrap_or("");
    let disk_part = parts.next().unwrap_or("");
    ServerSystemInfo {
        memory: parse_memory_sysctl(mem_part),
        disk: parse_disk_usage(disk_part),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "  PID %CPU %MEM    RSS COMMAND\n\
         1234  2.3  1.5  20480 game\n\
         5678  0.0  0.1   1024 sshd\n\
         9012 15.7  8.2 168000 db\n";

    #[test]
    fn filters_by_configured_process_names() {
        let names = vec!["game".to_string(), "db".to_string()];
        let result = parse_and_filter(SAMPLE, &names);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pid, 1234);
        assert_eq!(result[0].command, "game");
        assert_eq!(result[1].pid, 9012);
        assert_eq!(result[1].rss_kb, 168000);
    }

    #[test]
    fn empty_name_list_keeps_everything() {
        let result = parse_and_filter(SAMPLE, &[]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn matching_is_case_insensitive_substring() {
        let names = vec!["GAME".to_string()];
        let result = parse_and_filter(SAMPLE, &names);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].command, "game");
    }

    #[test]
    fn ignores_malformed_lines_instead_of_failing() {
        let broken = "  PID %CPU %MEM    RSS COMMAND\nnot a valid line\n";
        assert!(parse_and_filter(broken, &[]).is_empty());
    }

    #[test]
    fn builds_system_info_command_with_escaped_path() {
        let cmd = build_system_info_command("/usr/home/game");
        assert_eq!(
            cmd,
            "sysctl -n hw.physmem hw.pagesize vm.stats.vm.v_free_count; echo '---M2MANAGER-DISK---'; df -Pk '/usr/home/game'"
        );
    }

    const SYSTEM_INFO_SAMPLE: &str = "34359738368\n4096\n1048576\n\
         ---M2MANAGER-DISK---\n\
         Filesystem  1024-blocks     Used    Avail Capacity Mounted on\n\
         /dev/da0s1a    104857600 41943040 58309936      42% /usr/home\n";

    #[test]
    fn parses_combined_memory_and_disk_output() {
        let info = parse_system_info_output(SYSTEM_INFO_SAMPLE);
        let memory = info.memory.expect("memory should parse");
        assert_eq!(memory.total_bytes, 34359738368);
        assert_eq!(memory.free_bytes, 1048576 * 4096);

        let disk = info.disk.expect("disk should parse");
        assert_eq!(disk.total_kb, 104857600);
        assert_eq!(disk.used_kb, 41943040);
        assert_eq!(disk.avail_kb, 58309936);
        assert_eq!(disk.capacity_percent, 42);
        assert_eq!(disk.mount_point, "/usr/home");
    }

    #[test]
    fn missing_disk_marker_still_parses_memory() {
        let info = parse_system_info_output("34359738368\n4096\n1048576\n");
        assert!(info.memory.is_some());
        assert!(info.disk.is_none());
    }
}
