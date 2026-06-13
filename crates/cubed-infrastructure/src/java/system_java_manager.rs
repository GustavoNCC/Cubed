use async_trait::async_trait;
use tokio::process::Command;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{JavaInstallation, JavaManager};

/// Candidatos donde buscar Java en el sistema.
/// En Ubuntu/Linux también se consulta `which java` y JAVA_HOME.
const CANDIDATE_PATHS: &[&str] = &[
    "/usr/bin/java",
    "/usr/lib/jvm/default-java/bin/java",
    "/usr/lib/jvm/java-21-openjdk-amd64/bin/java",
    "/usr/lib/jvm/java-17-openjdk-amd64/bin/java",
    "/usr/lib/jvm/java-11-openjdk-amd64/bin/java",
    "/usr/lib/jvm/java-8-openjdk-amd64/jre/bin/java",
    // macOS (desarrollo)
    "/opt/homebrew/opt/openjdk@21/bin/java",
    "/opt/homebrew/opt/openjdk@17/bin/java",
    "/opt/homebrew/opt/openjdk@11/bin/java",
    "/usr/local/opt/openjdk@21/bin/java",
];

/// Versión mayor mínima de Java requerida por versión de Minecraft.
fn min_java_for_minecraft(mc_version: &str) -> u32 {
    let parts: Vec<u32> = mc_version
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    let (minor, patch) = match parts.as_slice() {
        [_, minor, patch, ..] => (*minor, *patch),
        [_, minor] => (*minor, 0),
        _ => return 8,
    };

    // 1.20.5+ → Java 21
    if minor > 20 || (minor == 20 && patch >= 5) {
        return 21;
    }
    // 1.18+ → Java 17
    if minor >= 18 {
        return 17;
    }
    // 1.17+ → Java 16
    if minor >= 17 {
        return 16;
    }
    8
}

/// Extrae la versión mayor de Java de la salida de `java -version`.
/// La salida tiene la forma:
///   openjdk version "17.0.11" 2024-04-16
///   java version "1.8.0_412"
fn parse_major_version(output: &str) -> Option<u32> {
    // Busca la primera cadena entre comillas: "X.Y.Z" o "X"
    let quoted = output.lines().find_map(|line| {
        let start = line.find('"')? + 1;
        let end = line[start..].find('"')? + start;
        Some(&line[start..end])
    })?;

    let first: u32 = quoted.split('.').next()?.parse().ok()?;
    // Java 8 reporta "1.8.x" → versión mayor = 8
    if first == 1 {
        quoted.split('.').nth(1)?.parse().ok()
    } else {
        Some(first)
    }
}

async fn probe_java(path: &str) -> Option<JavaInstallation> {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .await
        .ok()?;

    // `java -version` escribe en stderr
    let text = String::from_utf8_lossy(&output.stderr).to_string();
    let major_version = parse_major_version(&text)?;

    let version_string = text.lines().next().unwrap_or("").trim().to_string();

    Some(JavaInstallation {
        path: path.to_string(),
        major_version,
        version_string,
    })
}

pub struct SystemJavaManager;

impl SystemJavaManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemJavaManager {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl JavaManager for SystemJavaManager {
    async fn detect_installations(&self) -> ApplicationResult<Vec<JavaInstallation>> {
        let mut found: Vec<JavaInstallation> = Vec::new();

        // 1. Candidatos estáticos
        for path in CANDIDATE_PATHS {
            if let Some(inst) = probe_java(path).await {
                if !found.iter().any(|f: &JavaInstallation| f.path == inst.path) {
                    found.push(inst);
                }
            }
        }

        // 2. `which java`
        if let Ok(out) = Command::new("which").arg("java").output().await {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() && !found.iter().any(|f| f.path == path) {
                if let Some(inst) = probe_java(&path).await {
                    found.push(inst);
                }
            }
        }

        // 3. JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let path = format!("{}/bin/java", java_home);
            if !found.iter().any(|f| f.path == path) {
                if let Some(inst) = probe_java(&path).await {
                    found.push(inst);
                }
            }
        }

        // Ordenar por versión mayor descendente
        found.sort_by(|a, b| b.major_version.cmp(&a.major_version));
        Ok(found)
    }

    async fn inspect(&self, path: &str) -> ApplicationResult<JavaInstallation> {
        probe_java(path).await.ok_or_else(|| {
            ApplicationError::Infrastructure(format!(
                "No se pudo obtener la versión de Java en '{}'", path
            ))
        })
    }

    fn validate_compatibility(
        &self,
        java: &JavaInstallation,
        minecraft_version: &str,
    ) -> ApplicationResult<()> {
        let required = min_java_for_minecraft(minecraft_version);
        if java.major_version < required {
            return Err(ApplicationError::Infrastructure(format!(
                "Minecraft {} requiere Java {} o superior, pero se encontró Java {}",
                minecraft_version, required, java.major_version
            )));
        }
        Ok(())
    }

    async fn select_for_version(
        &self,
        minecraft_version: &str,
    ) -> ApplicationResult<JavaInstallation> {
        let required = min_java_for_minecraft(minecraft_version);
        let installations = self.detect_installations().await?;

        // Preferir la versión mínima compatible (no necesariamente la más nueva)
        installations
            .into_iter()
            .filter(|j| j.major_version >= required)
            .min_by_key(|j| j.major_version)
            .ok_or_else(|| {
                ApplicationError::Infrastructure(format!(
                    "No se encontró Java {} o superior en el sistema. \
                     Instala OpenJDK con: sudo apt install openjdk-{}-jdk",
                    required, required
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_java_for_old_minecraft_is_8() {
        assert_eq!(min_java_for_minecraft("1.16.5"), 8);
    }

    #[test]
    fn min_java_for_1_17_is_16() {
        assert_eq!(min_java_for_minecraft("1.17.1"), 16);
    }

    #[test]
    fn min_java_for_1_18_is_17() {
        assert_eq!(min_java_for_minecraft("1.18.2"), 17);
    }

    #[test]
    fn min_java_for_1_20_5_is_21() {
        assert_eq!(min_java_for_minecraft("1.20.5"), 21);
    }

    #[test]
    fn min_java_for_1_21_is_21() {
        assert_eq!(min_java_for_minecraft("1.21.4"), 21);
    }

    #[test]
    fn parse_modern_java_version() {
        let out = r#"openjdk version "21.0.3" 2024-04-16
OpenJDK Runtime Environment (build 21.0.3+9-Ubuntu-1ubuntu122.04)
OpenJDK 64-Bit Server VM (build 21.0.3+9-Ubuntu-1ubuntu122.04, mixed mode, sharing)"#;
        assert_eq!(parse_major_version(out), Some(21));
    }

    #[test]
    fn parse_java_8_version() {
        let out = r#"java version "1.8.0_412"
Java(TM) SE Runtime Environment (build 1.8.0_412-b08)
Java HotSpot(TM) 64-Bit Server VM (build 25.412-b08, mixed mode)"#;
        assert_eq!(parse_major_version(out), Some(8));
    }

    #[test]
    fn parse_java_17_version() {
        let out = r#"openjdk version "17.0.11" 2024-04-16
OpenJDK Runtime Environment (build 17.0.11+9-Ubuntu-122.04.1)
OpenJDK 64-Bit Server VM (build 17.0.11+9-Ubuntu-122.04.1, mixed mode, sharing)"#;
        assert_eq!(parse_major_version(out), Some(17));
    }

    #[test]
    fn validate_compatible_java() {
        let mgr = SystemJavaManager::new();
        let java = JavaInstallation {
            path: "/usr/bin/java".into(),
            major_version: 21,
            version_string: "openjdk 21".into(),
        };
        assert!(mgr.validate_compatibility(&java, "1.21.4").is_ok());
    }

    #[test]
    fn validate_incompatible_java() {
        let mgr = SystemJavaManager::new();
        let java = JavaInstallation {
            path: "/usr/bin/java".into(),
            major_version: 11,
            version_string: "openjdk 11".into(),
        };
        assert!(mgr.validate_compatibility(&java, "1.21.4").is_err());
    }

    #[tokio::test]
    async fn detect_finds_something_or_empty() {
        let mgr = SystemJavaManager::new();
        // No debe paniquear aunque no haya Java instalado
        let result = mgr.detect_installations().await;
        assert!(result.is_ok());
    }
}
