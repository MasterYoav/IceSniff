def print_icesniff():
    RESET = "\033[0m"

    def fg(r, g, b):
        return f"\033[38;2;{r};{g};{b}m"

    FONT = {
        "I": [
            "██████",
            "  ██  ",
            "  ██  ",
            "  ██  ",
            "  ██  ",
            "  ██  ",
            "██████",
        ],
        "C": [
            " █████",
            "██   █",
            "██    ",
            "██    ",
            "██    ",
            "██   █",
            " █████",
        ],
        "E": [
            "██████",
            "██    ",
            "██    ",
            "█████ ",
            "██    ",
            "██    ",
            "██████",
        ],
        "S": [
            " █████",
            "██    ",
            "██    ",
            " ████ ",
            "    ██",
            "    ██",
            "█████ ",
        ],
        "N": [
            "██  ██",
            "██  ██",
            "███ ██",
            "██████",
            "██ ███",
            "██  ██",
            "██  ██",
        ],
        "F": [
            "██████",
            "██    ",
            "██    ",
            "█████ ",
            "██    ",
            "██    ",
            "██    ",
        ],
    }

    text = "ICESNIFF"

    colors = [
        (18, 76, 150),
        (28, 102, 176),
        (42, 128, 198),
        (62, 156, 216),
        (92, 186, 231),
        (128, 211, 241),
        (176, 231, 248),
        (226, 245, 252),
    ]

    def shadow_from_color(rgb):
        r, g, b = rgb
        return fg(max(0, r - 42), max(0, g - 44), max(0, b - 52))

    def glyph_mask(glyph):
        return [[c != " " for c in row] for row in glyph]

    def build_letter(glyph_rows, main_rgb):
        main_color = fg(*main_rgb)
        shadow_color = shadow_from_color(main_rgb)

        mask = glyph_mask(glyph_rows)
        h = len(mask)
        w = len(mask[0])

        cw = w + 1
        ch = h + 1

        chars = [[" " for _ in range(cw)] for _ in range(ch)]
        cols = [["" for _ in range(cw)] for _ in range(ch)]

        for y in range(h):
            for x in range(w):
                if not mask[y][x]:
                    continue

                right_edge = x + 1 >= w or not mask[y][x + 1]
                bottom_edge = y + 1 >= h or not mask[y + 1][x]

                if right_edge and x + 1 < cw and chars[y][x + 1] == " ":
                    chars[y][x + 1] = "│"
                    cols[y][x + 1] = shadow_color

                if bottom_edge and y + 1 < ch and chars[y + 1][x] == " ":
                    chars[y + 1][x] = "─"
                    cols[y + 1][x] = shadow_color

                if right_edge and bottom_edge and x + 1 < cw and y + 1 < ch:
                    chars[y + 1][x + 1] = "┘"
                    cols[y + 1][x + 1] = shadow_color

        for y in range(h):
            for x in range(w):
                if mask[y][x]:
                    chars[y][x] = "█"
                    cols[y][x] = main_color

        rendered = []
        for y in range(ch):
            row = []
            for x in range(cw):
                ch_ = chars[y][x]
                if ch_ == " ":
                    row.append(" ")
                else:
                    row.append(f"{cols[y][x]}{ch_}{RESET}")
            rendered.append("".join(row))
        return rendered, cw

    rendered_letters = []
    widths = []

    for i, ch in enumerate(text):
        letter_rows, width = build_letter(FONT[ch], colors[i])
        rendered_letters.append(letter_rows)
        widths.append(width)

    total_rows = max(len(letter) for letter in rendered_letters)

    print()
    for r in range(total_rows):
        parts = []
        for i, letter in enumerate(rendered_letters):
            if r < len(letter):
                part = letter[r]
            else:
                part = " " * widths[i]
            parts.append(part)
        print(" ".join(parts))
    print()


if __name__ == "__main__":
    print_icesniff()
