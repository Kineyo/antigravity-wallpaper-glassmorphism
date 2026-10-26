# Análisis del DOM y Tailwind CSS en Antigravity IDE

## Estructura Jerárquica del DOM

A través de la inspección en vivo con Chrome DevTools Protocol (`Runtime.evaluate`), determinamos los componentes estructurales clave:

```html
<body style="background-image: url(...) !important;">
  <div class="h-screen w-screen flex flex-col bg-background text-foreground">
    <!-- Header superior / Toolbar (h=35px) -->
    <div class="shrink-0">
      <div class="flex items-center gap-1 px-2 h-full">...</div>
    </div>
    
    <!-- Contenedor de Trabajo (h=restante) -->
    <div class="flex-1 flex min-h-0 relative">
      <!-- Sidebar / Explorador (w=256px) -->
      <div class="bg-sidebar">...</div>
      
      <!-- Divisor (w=1px) -->
      <div class="bg-border">...</div>
      
      <!-- Área Central: Chat / Editor / Terminal (w=restante) -->
      <div class="flex-1 flex min-h-0">
        <div class="bg-card">...</div>
      </div>
    </div>
  </div>
</body>
```

## Selectores Críticos para Glassmorphism

| Elemento | Clases Tailwind | Propiedad Inyectada | Efecto Logrado |
|---|---|---|---|
| **Contenedor Raíz** | `.bg-background` | `background-color: transparent; background-image: url(...) fixed cover;` | Revela el wallpaper completo |
| **Sidebar** | `.bg-sidebar, [class*="bg-sidebar"]` | `background-color: rgba(12, 14, 20, 0.72); backdrop-filter: blur(14px);` | Panel lateral translúcido |
| **Tarjetas / Chat** | `.bg-card, [class*="bg-card"]` | `background-color: rgba(16, 18, 24, 0.68); backdrop-filter: blur(10px);` | Mensajes y tarjetas con efecto glass |
| **Header** | `.h-screen > .shrink-0 > div` | `background-color: rgba(10, 12, 16, 0.80); backdrop-filter: blur(16px);` | Barra superior sutil e integrada |
| **Área Central** | `.flex-1.flex.min-h-0 > div` | `background-color: rgba(12, 14, 18, 0.60); backdrop-filter: blur(8px);` | Fondo del editor y visualizador |
| **Bordes** | `.bg-border, .border-border` | `border-color: rgba(255, 255, 255, 0.08);` | Líneas divisoras sutiles tipo cristal |
| **Contenido** | `p, span, pre, code, button, input` | `opacity: 1 !important;` | **Protección absoluta de legibilidad** |

## El Descubrimiento del Límite de Custom Properties

Cuando se utilizó una variable CSS (`:root { --agy-wallpaper-url: url("data:image/png;base64,...") }`) con una imagen de 2.5 MB (más de 3.3 millones de caracteres en base64), el parser de Blink/Chromium **descartó silenciosamente la variable**. 

**Regla de oro descubierta:** La URL de imágenes base64 en Electron debe colocarse **directamente** en la regla del selector (`background-image: url(...)`), o bien optimizar la imagen en memoria a formato JPEG 93% (~400-600 KB) para garantizar un renderizado instantáneo y un uso mínimo de memoria.
