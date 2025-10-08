# Test C# coordinate formula
screen_width = 800
screen_height = 600
cell_width = 48
cell_height = 32

offset_x = screen_width // 2 // cell_width  # 8
offset_y = screen_height // 2 // cell_height - 1  # 8

print(f"OffSetX = {offset_x}, OffSetY = {offset_y}\n")

player_x = 100
player_y = 100

print("Tiles around player (100, 100):")
print("=" * 60)
for dy in range(-2, 3):
    for dx in range(-2, 3):
        tile_x = player_x + dx
        tile_y = player_y + dy
        
        # C# formula
        draw_x = (tile_x - player_x + offset_x) * cell_width - offset_x
        draw_y = (tile_y - player_y + offset_y) * cell_height
        
        marker = " <-- PLAYER" if (dx == 0 and dy == 0) else ""
        print(f"Tile({tile_x:3}, {tile_y:3}) -> Draw({draw_x:4}, {draw_y:4}){marker}")
    print()

print("\nScreen center: (400, 300)")
print("Player tile should be at: (376, 256)")
print("Offset from center: (-24, -44)")
