precision mediump float;

uniform float time;

// Resolution and canvas element size don't match and I have not figured out how to make them match.
// In fact the canvas is not rendering at full display resolution right now, but it's probably
// good anyway to keep the GPU usage down.
uniform vec2 resolution;
uniform vec2 element_size;

// In screen coordinates with origin at the top left of the canvas, same scale as
// element_size (but not resolution!)
uniform vec2 cursor;

#define M_PI 3.14159265358979323846

float alignment_from_center(vec2 a, vec2 b, vec2 rectangle) {
    vec2 center = rectangle / 2.0;
    return pow(clamp(dot(normalize(a - center), normalize(b - center)), 0.0, 1.0), 4.0);
}

vec3 brighten(vec3 color, float factor) {
    float exponent = (1.0 / (factor + 1.0));
    return vec3(pow(color.r, exponent), pow(color.g, exponent), pow(color.b, exponent));
}

void main() {
    vec2 ratio = vec2(1.0, resolution.y / resolution.x);

    vec2 relative_position = gl_FragCoord.xy / resolution;

    vec2 uv = relative_position * ratio;

    vec2 centre = ratio / 2.0;

    vec2 dist = uv - centre;

    vec2 screen_position = relative_position * element_size;
    // Invert Y for cursor to match shader coordinates
    vec2 inverted_cursor = vec2(cursor.x, element_size.y -cursor.y);

    float alignment = alignment_from_center(inverted_cursor, screen_position, element_size);
    
    float cursor_factor = sqrt(1.0 / (length(screen_position - inverted_cursor) + 1.0)) + 0.01 * alignment;

    float dist_rad = sqrt(sqrt(pow(abs(dist.x), 2.0 + cursor_factor) + pow(abs(dist.y), 2.0 + cursor_factor))) * M_PI * 3.0;

    float intensity = (sin(dist_rad * 13.0 + time * 12.0) + 1.0) / 2.0;

    float polar = atan(dist.x, dist.y) + time;

    float intensity_2 = pow(sin(polar * (10.0) - dist_rad * 13.0), 2.0);

    float intensity_3 = clamp(dist_rad / 2.0, 0.0, 1.0);

    vec3 final = vec3(uv, 0.5 + 0.5 * sin(time)) * intensity * intensity_2 * intensity_3;

    gl_FragColor = vec4(brighten(final, alignment), 1.0);
}
