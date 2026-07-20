# NSIS Installer Images

The Windows installer requires two custom bitmap images for branding. These are optional - if not provided, NSIS will use default images.

## Required Images

### 1. Header Image (`installer-header.bmp`)
- **Location:** `src-tauri/installer-header.bmp`
- **Dimensions:** 150 x 57 pixels
- **Format:** BMP (24-bit)
- **Usage:** Appears at the top of the installer wizard pages

### 2. Sidebar Image (`installer-sidebar.bmp`)
- **Location:** `src-tauri/installer-sidebar.bmp`
- **Dimensions:** 164 x 314 pixels
- **Format:** BMP (24-bit)
- **Usage:** Appears on the left side of the welcome and completion pages

## Creating the Images

You can create these images using any image editor (Photoshop, GIMP, Paint.NET, etc.):

1. Design your branded images with the exact dimensions above
2. Export as BMP format (24-bit color depth)
3. Save them in the `src-tauri/` directory with the exact filenames above

## Temporary Workaround

If you want to build the installer immediately without custom images, you can:

1. Remove the `headerImage` and `sidebarImage` lines from `src-tauri/tauri.conf.json`
2. NSIS will use its default blue gradient images

Or create simple placeholder images:
- Use a solid color or simple gradient
- Add your app name/logo as text
- Export as BMP with the correct dimensions

## Example Tools

- **Windows:** Paint, Paint.NET, GIMP
- **Online:** Photopea (photopea.com) - free online Photoshop alternative
- **Command line:** ImageMagick can convert and resize images to BMP format
