# macOS 本机签名与升级权限

## 免费方案的选择

| 方式 | 费用 | 更新时的代码身份 | 使用范围 |
|---|---|---|---|
| ad-hoc（`-`） | 免费 | 随构建哈希变化 | 临时测试,可能重复授权 |
| 固定自签代码签名证书 | 免费 | 使用同一证书与 bundle ID 时可保持稳定 | 自己的 Mac / 内部测试,无 Apple 公证 |
| Developer ID Application | 需要 Apple Developer Program | 固定团队与 bundle ID | 面向公众分发,可接入公证 |

Apple 通过 designated requirement 识别代码身份。ad-hoc 身份绑定某次构建;固定自签证书可提供基于证书的身份,但不提供 Gatekeeper 的 Apple 开发者认可。稳定身份是避免升级丢失授权的条件,不能代替实际版本替换验收。参考 [Apple TN3127](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements/) 与 [Apple TN2206](https://developer.apple.com/library/archive/technotes/tn2206/)。

## 本机自签

1. 在「钥匙串访问 → 证书助理 → 创建证书」中,使用固定名称（例如 `Lenz Local Development`）,身份类型选「自签名根」,证书类型选「代码签名」。保存并持续复用此证书及私钥,不要每次构建重新生成。
2. 证书创建、钥匙串访问/代码签名信任提示由用户确认。此构建入口不修改证书信任,也不会重置辅助功能或屏幕录制授权。
3. 创建固定身份后,本机可将公开证书指纹保存在 `~/Library/Application Support/Lenz/Signing/identity.json` 的 `identity` 字段。当前本机已保存此记录,证书名为 `Lenz Local Development`,私钥留在登录钥匙串,有效期到 2036-11-02。从仓库根目录执行:

   ```sh
   pnpm macos:local
   ```

   脚本优先复用本机保存的 SHA-1 指纹,避免环境中其他签名身份覆盖它。记录损坏或对应证书/私钥不可用时直接失败,不重新生成或回退 ad-hoc。其他机器也可显式执行 `pnpm macos:local "Lenz Local Development"` 或传 `security find-identity -p codesigning` 列出的 SHA-1 身份。构建不导入证书、不尝试公证;产物通过严格签名及指定证书的 designated requirement 校验后才成功。
4. 输出为 `src-tauri/target/release/bundle/macos/Lenz.app`（设置 Cargo 目标目录/目标平台时随之调整）。退出旧应用后,手动替换 `/Applications/Lenz.app`,保持 `app.lenz.desktop` 和安装位置一致。从旧 ad-hoc 身份首次切换到自签身份时,需要重新授予权限一次。
5. 下一次更新继续用同一证书构建并替换,检查 `AXIsProcessTrusted`、屏幕录制及常用源应用取词。证书被重新生成、更换签名方式、系统升级等情形需要重新验收。本机替换验收结果见下方记录。

私钥和 `.p12` 备份不要加入仓库。当前构建入口不会覆盖已安装应用或上传任何证书。

## 2026-11-05 本机替换验收

- 环境:macOS 27.2 / Apple Silicon,固定安装路径 `/Applications/Lenz.app`,bundle ID `app.lenz.desktop`。
- 两份 0.1.4 release 构建的二进制 SHA256 和 CDHash 不同,均由同一固定自签证书签署,designated requirement 完全一致。
- 首次从旧 ad-hoc 切换后,通过系统界面重新添加当前应用的设备控制和录屏条目;系统认证由用户完成。录屏旧条目虽然显示开启,进程仍未获得权限,移除旧条目并重新添加后生效。
- 两项权限均生效后,执行 Build B → Build A → Build B 往返替换,期间没有再修改系统授权。每次新进程的「通用 → 重新检查」均未显示缺少设备控制或录屏权限,即对应的 `AXIsProcessTrusted` 与 `CGPreflightScreenCaptureAccess` 仍为 true;没有再次出现授权请求。
- 最终安装 Build B,原后台启动偏好已恢复。签名 ZIP 解压后通过严格签名验证,二进制与最终安装包一致;验收数据和两个构建保留在本机 `src-tauri/target/local-signed/verification/`。

结论覆盖这台 Mac、当前系统、同一证书及相同安装路径下的两份不同构建,不包含其他系统版本或证书更换后的授权保持。

## CI 的可选正式签名

在发布仓库 secrets 中配置:

- `APPLE_CERTIFICATE`:固定 Developer ID Application `.p12` 的 Base64 内容。
- `APPLE_CERTIFICATE_PASSWORD`:该导出文件的密码。
- `APPLE_SIGNING_IDENTITY`:完整证书名称,如 `Developer ID Application: Example (AB12345678)`。

三项全部缺失时继续生成现有 ad-hoc 测试 DMG,并警告更新可能再次要求授权。只配置部分凭据会直接失败。完整配置时由 Tauri 导入并签名,产物再验证 Authority、TeamIdentifier 和 designated requirement。公证仍需额外 Apple 凭据,不因签名成功而视为已公证。相关配置参见 [Tauri macOS 签名文档](https://v2.tauri.app/distribute/sign/macos/)。
