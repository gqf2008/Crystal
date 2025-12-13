#version 100
varying lowp vec4 color;
varying lowp vec2 uv;

uniform sampler2D Texture;

// x1, y1, x2, y2 in screen pixels (OpenGL coords: origin bottom-left)
// 需要一定精度表示屏幕像素坐标，lowp 可能导致量化/溢出。
uniform mediump vec4 FocusRect;
uniform lowp float FocusAlpha;

void main() {
    lowp vec4 c = color * texture2D(Texture, uv);

    mediump vec2 p = gl_FragCoord.xy;

    // 由 FocusRect 派生一个“人形轮廓”（竖向胶囊 / 圆角矩形）。
    // FocusRect 坐标系：OpenGL 屏幕坐标（左下原点）。
    // 胶囊宽度取 rect 的半宽，胶囊高度为 rect 高度；上下圆角半径 = 半宽。
    if (p.x >= FocusRect.x && p.x <= FocusRect.z && p.y >= FocusRect.y && p.y <= FocusRect.w) {
        mediump vec2 center = 0.5 * (FocusRect.xy + FocusRect.zw);
        mediump vec2 halfSize = 0.5 * (FocusRect.zw - FocusRect.xy);

        mediump float r = max(1.0, halfSize.x);
        mediump float h = max(0.0, halfSize.y - r);

        // 胶囊端点（竖直方向）
        mediump vec2 a = vec2(0.0, -h);
        mediump vec2 b = vec2(0.0,  h);

        // Signed distance to capsule (centered, vertical)
        mediump vec2 q = p - center;
        mediump vec2 pa = q - a;
        mediump vec2 ba = b - a;
        mediump float t = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
        mediump float dist = length(pa - ba * t) - r;

        // Feather 边缘，避免硬边锯齿
        mediump float feather = 1.5;
        mediump float mask = 1.0 - smoothstep(0.0, feather, dist);

        // mask=1 表示完全在轮廓内：alpha 乘 FocusAlpha
        // mask=0 表示完全在轮廓外：保持原 alpha
        mediump float aMul = mix(1.0, clamp(FocusAlpha, 0.0, 1.0), mask);
        c.a *= aMul;
    }

    gl_FragColor = c;
}
