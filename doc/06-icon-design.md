# 图标资源

使用内置 imagegen 生成，未使用 API/CLI 回退。

- 应用图标源文件：`src/assets/app-icon.png`，白色圆角底，青蓝、紫色和薄荷色上升折带。
- macOS 菜单栏图标：`src/assets/tray-template-source.png` 为内置 imagegen 生成的透明底单色源图，Tauri CLI 生成 `src-tauri/icons/tray-template.png`（44px）；启用系统模板模式，随系统明暗外观呈现黑色或白色。
- Windows/Linux 托盘继续使用彩色 `src-tauri/icons/tray.png`；应用与界面图标仍为白底彩色。
- 各平台应用图标由 `npm run tauri -- icon src/assets/app-icon.png` 生成。

## 应用图标最终提示词

Edit this exact app icon. Keep its recognizable folded ascending A / arrow shape and centered rounded-square composition. Replace the dark forest-green tile with a clean WHITE rounded-square tile. Recolor the ribbon itself with bright, fresh cyan, sky blue and lively violet, with a small mint highlight. Use light, crisp colors and restrained soft depth. This should feel like a friendly modern macOS utility, airy and clean, never dark or somber. White background INSIDE the tile; genuine transparent background OUTSIDE the rounded tile. Keep clean icon-safe padding. No text, no mockup, no extra decorations, no black or deep-green surfaces. Standalone square production app icon.


## 菜单栏图标提示词

Create the macOS menu bar template version of this exact application icon. Preserve the recognisable folded A / upward arrow silhouette. Remove the green rounded-square tile entirely. Only a single SOLID PURE BLACK symbol with actual transparent background, no gray, no green, no ivory, no texture, no lighting, no shadow. Simplify folds into clean negative-space cuts so the mark is readable at 18px. Center the symbol in a square image with 16 percent transparent padding. This is a production monochrome template tray icon, not a mockup. No text.
