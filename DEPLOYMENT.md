# 🚀 xpose Deployment Guide

This guide ensures you have everything set up to leverage the automated CI/CD pipeline of **xpose**.

## 🛠️ GitHub Secrets Configuration

To enable automated testing and professional-grade deployment, you must add the following secrets to your GitHub repository (**Settings > Secrets and variables > Actions**):

| Secret Name | Purpose | How to generate |
| :--- | :--- | :--- |
| `NPM_TOKEN` | Automated NPM publishing | [npmjs.com](https://www.npmjs.com/) > Access Tokens > **Automation Type** |
| `CLOUDFLARE_API_WORKER_TOKEN` | Auto-deploy Key Server (CI/CD) | [dash.cloudflare.com](https://dash.cloudflare.com/) > Profile > API Tokens > **Edit Workers** |
| `CLOUDFLARE_API_TUNNEL_TOKEN` | Dynamic tunnel provisioning at runtime | [dash.cloudflare.com](https://dash.cloudflare.com/) > Profile > API Tokens > **Cloudflare Tunnel: Edit** |

### 🔍 Token Permissions (Cloudflare)

**`CLOUDFLARE_API_WORKER_TOKEN`** — used by CI/CD to deploy the Worker:
- **Account / Workers Scripts / Edit**
- **Account / D1 / Edit**
- **User / Memberships / Read**

**`CLOUDFLARE_API_TUNNEL_TOKEN`** — used at runtime by the Worker to provision tunnels dynamically (optional):
- **Account / Cloudflare Tunnel / Edit**
- **Zone / DNS / Edit** (for routing DNS records)

---

## 📦 Publishing Flow

### 1. Version Update
Update the version in both `packages/cli/Cargo.toml` and `packages/cli/package.json`.

### 2. Triggering Release
The `release.yml` workflow triggers automatically when you push a version tag:

```bash
git tag v0.1.x
git push origin v0.1.x
```

### 3. What happens next?
1. **Lint/Test**: `ci.yml` verifies code quality.
2. **Build**: GitHub Actions builds multi-platform binaries using Docker.
3. **Deploy**: The Key Server is updated on Cloudflare.
4. **Publish**: The New CLI version is pushed to NPM.

---

## 🧹 Maintenance

### Logs
CLI logs are stored locally at:
- Linux/macOS: `~/.xpose/logs/xpose.log`
- Windows: `%USERPROFILE%\.xpose\logs\xpose.log`

### Binary Cache
Calculated binaries and Cloudflare binaries are cached in:
- `~/.xpose/bin/`
