# AitoType Design System (Apple Fluid)

## Design Philosophy

**"Liquid Glass"**
Inspired by visionOS and macOS Sonoma. Focus on depth, fluid materials, and extremely refined details. The interface should simulate physical glass and liquid metal materials.

## Color Palette

### Materials
- **App Background**: Fluid Gradient (Subtle Blue/Teal/Purple blend, very dark/light depending on theme).
- **Glass Base**: `rgba(255, 255, 255, 0.6)` (Light) / `rgba(30, 30, 30, 0.4)` (Dark).
- **Sidebar Material**: `rgba(240, 240, 240, 0.5)` / `vibrancy-sidebar` (macOS native).

### Accents (Apple Style)
- **Fluid Blue**: `#007AFF`
- **Liquid Silver**: `linear-gradient(135deg, #E0E0E0, #F5F5F5)`
- **Active State**: `#0A84FF` (Dark Mode Blue)
- **Destructive**: `#FF453A`

## Typography

- **Font**: System UI (`-apple-system`)
- **Tracking**: Slightly loose for headings, normal for body.
- **Weights**:
  - Light (300) for large display numbers.
  - Regular (400) for body.
  - Medium (500) for buttons.

## Layout Structure

**Split View**:
- **Left Sidebar (260px)**: Navigation, History List, Settings Toggle.
- **Right Content (Flex)**: Recording Visualization, Transcript Result.

## Components

### Liquid Button
- Not just a circle. A "droplet" of liquid metal or glass.
- **Interaction**: On hover, it morphs slightly. On click, it ripples.

### Glass Card
- High blur (`backdrop-filter: blur(40px)`).
- Distinct white border (0.5px) with low opacity.
- Soft highlight at the top edge.

### Waveform
- Organic lines, not rigid bars.
- Smooth aesthetic, slower movement.
