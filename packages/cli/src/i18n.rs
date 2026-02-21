use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    En,
    Vi,
    Zh,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vi" => Language::Vi,
            "zh" | "cn" => Language::Zh,
            _ => Language::En,
        }
    }

    pub fn auto_detect() -> Self {
        let lang = env::var("LANG")
            .or_else(|_| env::var("LC_ALL"))
            .unwrap_or_default();
        if lang.contains("vi") {
            Language::Vi
        } else if lang.contains("zh") || lang.contains("CN") {
            Language::Zh
        } else {
            Language::En
        }
    }
}

pub struct I18n {
    pub lang: Language,
}

impl I18n {
    pub fn new(lang_override: Option<String>) -> Self {
        let lang = lang_override
            .map(|s| Language::from_str(&s))
            .unwrap_or_else(Language::auto_detect);
        Self { lang }
    }

    pub fn t(&self, key: &'static str) -> &'static str {
        match self.lang {
            Language::Vi => self.get_vi(key),
            Language::Zh => self.get_zh(key),
            Language::En => self.get_en(key),
        }
    }

    fn get_en(&self, key: &'static str) -> &'static str {
        match key {
            "startup" => "Starting xpose CLI",
            "connected" => "Connected",
            "tunnel_allocated" => "Tunnel allocated.",
            "auto_detecting" => "Auto-detecting local port...",
            "detected_port" => "Detected port",
            "no_port_found" => "No active port found. Please specify one.",
            "dashboard_title" => " xpose dashboard - Monitoring Hub",
            "global_stats" => "Global: {} Busy / {} Available",
            "active_tunnels" => " Active Tunnels ",
            "tunnel_details" => " Tunnel Details ",
            "infra_usage" => " Global Infrastructure Usage ",
            "footer_help" => " [Q] Quit  [↑/↓] Navigate",
            "release_tunnel" => "Releasing tunnel...",
            "error_collision" => "Port collision, please retry",
            "error_timeout" => "Request timed out.",
            "usage_limit" => " Usage (vs 1GB Limit) ",
            "scanning_ports" => "No port specified. Scanning common ports (3000, 8000, 8080)...",
            "found_service" => "Found active service on port {}",
            "checking_env" => "Checking environment...",
            "downloading_binary" => "Downloading cloudflared binary...",
            "installed_success" => "Cloudflared installed successfully.",
            "binary_found" => "Cloudflared binary found.",
            "version_outdated" => "Critical: Your CLI version (v{}) is outdated. Minimum required: v{}. Please update.",
            "update_available" => "Update available: v{} (Current: v{}). Please run 'npm update -g xpose-cli' soon.",
            "requesting_tunnel" => "Requesting tunnel from pool...",
            "running_background" => "cloudflared is running in background. Tunnel token hidden for security.",
            _ => key,
        }
    }

    fn get_vi(&self, key: &'static str) -> &'static str {
        match key {
            "startup" => "Đang khởi động xpose CLI",
            "connected" => "Đã kết nối",
            "tunnel_allocated" => "Đã cấp phát tunnel.",
            "auto_detecting" => "Đang tự động dò tìm port...",
            "detected_port" => "Đã tìm thấy port",
            "no_port_found" => "Không tìm thấy port nào đang chạy. Vui lòng chỉ định một port.",
            "dashboard_title" => " xpose dashboard - Trung tâm giám sát",
            "global_stats" => "Toàn cầu: {} Đang bận / {} Sẵn sàng",
            "active_tunnels" => " Danh sách Tunnel đang chạy ",
            "tunnel_details" => " Chi tiết Tunnel ",
            "infra_usage" => " Mức độ sử dụng hạ tầng toàn cầu ",
            "footer_help" => " [Q] Thoát  [↑/↓] Di chuyển",
            "release_tunnel" => "Đang giải phóng tunnel...",
            "error_collision" => "Trùng lặp port, vui lòng thử lại",
            "error_timeout" => "Yêu cầu hết thời gian chờ.",
            "usage_limit" => " Mức sử dụng (so với giới hạn 1GB) ",
            "scanning_ports" => "Không có port được chỉ định. Đang quét các port phổ biến (3000, 8000, 8080)...",
            "found_service" => "Tìm thấy dịch vụ đang hoạt động trên port {}",
            "checking_env" => "Đang kiểm tra môi trường...",
            "downloading_binary" => "Đang tải xuống binary cloudflared...",
            "installed_success" => "Đã cài đặt Cloudflared thành công.",
            "binary_found" => "Đã tìm thấy binary Cloudflared.",
            "version_outdated" => "Nghiêm trọng: Phiên bản CLI của bạn (v{}) đã cũ. Yêu cầu tối thiểu: v{}. Vui lòng cập nhật.",
            "update_available" => "Có phiên bản mới: v{} (Hiện tại: v{}). Vui lòng chạy 'npm update -g xpose-cli' sớm.",
            "requesting_tunnel" => "Đang yêu cầu tunnel từ pool...",
            "running_background" => "cloudflared đang chạy ngầm. Token tunnel được ẩn để bảo mật.",
            _ => self.get_en(key),
        }
    }

    fn get_zh(&self, key: &'static str) -> &'static str {
        match key {
            "startup" => "正在启动 xpose CLI",
            "connected" => "已连接",
            "tunnel_allocated" => "隧道已分配。",
            "auto_detecting" => "正在自动检测本地端口...",
            "detected_port" => "检测到端口",
            "no_port_found" => "未找到活动端口。请指定一个。",
            "dashboard_title" => " xpose 控制面板 - 监控中心",
            "global_stats" => "全局: {} 繁忙 / {} 可用",
            "active_tunnels" => " 活动隧道 ",
            "tunnel_details" => " 隧道详情 ",
            "infra_usage" => " 全局基础设施使用情况 ",
            "footer_help" => " [Q] 退出  [↑/↓] 导航",
            "release_tunnel" => "正在释放隧道...",
            "error_collision" => "端口冲突，请重试",
            "error_timeout" => "请求超时。",
            "usage_limit" => " 使用量 (对比 1GB 限制) ",
            "scanning_ports" => "未指定端口。正在扫描常用端口 (3000, 8000, 8080)...",
            "found_service" => "在端口 {} 上找到活动服务",
            "checking_env" => "正在检查环境...",
            "downloading_binary" => "正在下载 cloudflared 二进制文件...",
            "installed_success" => "Cloudflared 安装成功。",
            "binary_found" => "已找到 Cloudflared 二进制文件。",
            "version_outdated" => "严重：您的 CLI 版本 (v{}) 已过时。最低要求：v{}。请更新。",
            "update_available" => {
                "有新版本可用：v{} (当前：v{})。请尽快运行 'npm update -g xpose-cli'。"
            }
            "requesting_tunnel" => "正在从池中请求隧道...",
            "running_background" => "cloudflared 正在后台运行。为了安全，隧道令牌已隐藏。",
            _ => self.get_en(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_detection() {
        let i18n = I18n::new(Some("vi".to_string()));
        assert_eq!(i18n.t("connected"), "Đã kết nối");

        let i18n_en = I18n::new(Some("en".to_string()));
        assert_eq!(i18n_en.t("connected"), "Connected");

        let i18n_zh = I18n::new(Some("zh".to_string()));
        assert_eq!(i18n_zh.t("connected"), "已连接");
    }

    #[test]
    fn test_i18n_key_consistency() {
        let keys = vec![
            "startup",
            "connected",
            "tunnel_allocated",
            "auto_detecting",
            "detected_port",
            "no_port_found",
            "dashboard_title",
            "global_stats",
            "active_tunnels",
            "tunnel_details",
            "infra_usage",
            "footer_help",
            "release_tunnel",
            "error_collision",
            "error_timeout",
            "usage_limit",
            "scanning_ports",
            "found_service",
            "checking_env",
            "downloading_binary",
            "installed_success",
            "binary_found",
            "version_outdated",
            "update_available",
            "requesting_tunnel",
            "running_background",
        ];

        let langs = vec![Language::En, Language::Vi, Language::Zh];

        for lang in langs {
            let i18n = I18n { lang };
            for key in &keys {
                let translated = i18n.t(key);
                assert_ne!(
                    translated, *key,
                    "Missing translation for key '{}' in language {:?}",
                    key, lang
                );
            }
        }
    }
}
