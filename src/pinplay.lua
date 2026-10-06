-- Pinplay window controls. Loaded by the pinplay binary.
local mp = require("mp")
local options = require("mp.options")

local settings = {
    margin = 24,
    position = "bottom-right",
}
options.read_options(settings, "pinplay")

-- Keep mpv from starting its own drag. That path freezes the cursor and then
-- delivers the mouse-up as a click, which pauses playback.
mp.set_property("input-dragging-deadzone", "100000")

local drag_threshold = 6
local edge_margin = 10
local pressed = false
local dragged = false
local swallow_press = false
local origin = nil

local sizes = {
    { label = "Nano", geometry = "96x54" },
    { label = "Tiny", geometry = "160x90" },
    { label = "Small", geometry = "320x180" },
    { label = "Medium", geometry = "480x270" },
    { label = "Large", geometry = "640x360" },
}

local overlay = mp.create_osd_overlay("ass-events")
local menu = nil

local function anchor()
    local margin = tonumber(settings.margin) or 24
    if settings.position == "top-left" then
        return string.format("+%d+%d", margin, margin)
    elseif settings.position == "top-right" then
        return string.format("-%d+%d", margin, margin)
    elseif settings.position == "bottom-left" then
        return string.format("+%d-%d", margin, margin)
    end
    return string.format("-%d-%d", margin, margin)
end

local function paused()
    return mp.get_property_bool("pause") == true
end

local function muted()
    return mp.get_property_bool("mute") == true
end

local function current_geometry()
    local width = mp.get_property_number("osd-width")
    local height = mp.get_property_number("osd-height")
    if not width or not height then
        return nil
    end
    return string.format("%dx%d", math.floor(width + 0.5), math.floor(height + 0.5))
end

local function items()
    if menu == "size" then
        local current = current_geometry()
        local list = {}
        for _, size in ipairs(sizes) do
            local prefix = current == size.geometry and "✓ " or "    "
            list[#list + 1] = {
                label = prefix .. size.label .. "  " .. size.geometry,
                run = function()
                    mp.commandv("set", "geometry", size.geometry .. anchor())
                    menu = nil
                end,
            }
        end
        list[#list + 1] = {
            label = "뒤로",
            run = function()
                menu = "root"
            end,
        }
        return list
    end

    return {
        {
            label = paused() and "재생" or "일시정지",
            run = function()
                mp.command("cycle pause")
                menu = nil
            end,
        },
        {
            label = muted() and "음소거 해제" or "음소거",
            run = function()
                mp.command("cycle mute")
                menu = nil
            end,
        },
        {
            label = "크기",
            run = function()
                menu = "size"
            end,
        },
        {
            label = "닫기",
            run = function()
                mp.command("quit")
            end,
        },
    }
end

local function row_at(y, count, height)
    if count < 1 or not height or height <= 0 then
        return 1
    end
    local row = math.floor((y or 0) / height * count) + 1
    if row < 1 then
        return 1
    end
    if row > count then
        return count
    end
    return row
end

local function render()
    if not menu then
        overlay:remove()
        return
    end

    local list = items()
    local width = mp.get_property_number("osd-width") or 320
    local height = mp.get_property_number("osd-height") or 180
    local pos = mp.get_property_native("mouse-pos") or {}
    local selected = row_at(pos.y, #list, height)
    local row_height = height / #list
    local ass = ""

    local function event(body)
        if ass ~= "" then
            ass = ass .. "\n"
        end
        ass = ass .. body
    end

    event(string.format(
        "{\\an7\\pos(0,0)\\p1\\c&H101010&\\1a&H33&}m 0 0 l %d 0 l %d %d l 0 %d{\\p0}",
        width,
        width,
        height,
        height
    ))

    for index, item in ipairs(list) do
        local y = math.floor((index - 1) * row_height)
        local text_color = "FFFFFF"
        if index == selected then
            event(string.format(
                "{\\an7\\pos(0,%d)\\p1\\c&HFFFFFF&\\1a&H00&}m 0 0 l %d 0 l %d %d l 0 %d{\\p0}",
                y,
                width,
                width,
                math.ceil(row_height),
                math.ceil(row_height)
            ))
            text_color = "101010"
        end
        local font_size = math.max(13, math.floor(row_height * 0.42))
        event(string.format(
            "{\\an5\\fnApple SD Gothic Neo\\bord0\\shad0\\fs%d\\1c&H%s&\\pos(%d,%d)}%s",
            font_size,
            text_color,
            math.floor(width / 2),
            math.floor(y + row_height / 2),
            item.label
        ))
    end

    overlay.res_x = width
    overlay.res_y = height
    overlay.data = ass
    overlay:update()
end

local function toggle_menu()
    if menu then
        menu = nil
    else
        menu = "root"
    end
    render()
end

local function activate()
    if not menu then
        mp.command("cycle pause")
        mp.osd_message(paused() and "일시정지" or "재생", 0.6)
        return
    end

    local list = items()
    local height = mp.get_property_number("osd-height") or 180
    local pos = mp.get_property_native("mouse-pos") or {}
    local item = list[row_at(pos.y, #list, height)]
    if item then
        item.run()
    end
    render()
end

local function pointer()
    local pos = mp.get_property_native("mouse-pos") or {}
    return pos.x or 0, pos.y or 0
end

local function near_edge(x, y)
    local width = mp.get_property_number("osd-width") or 0
    local height = mp.get_property_number("osd-height") or 0
    if width <= edge_margin * 2 or height <= edge_margin * 2 then
        return false
    end
    return x < edge_margin
        or y < edge_margin
        or x > width - edge_margin
        or y > height - edge_margin
end

local function track_drag()
    if not pressed or dragged or menu or not origin then
        return
    end
    local x, y = pointer()
    local dx = x - origin.x
    local dy = y - origin.y
    if dx * dx + dy * dy < drag_threshold * drag_threshold then
        return
    end
    dragged = true
    if not origin.edge then
        mp.command("begin-vo-dragging")
    end
end

local function finish_gesture(was_drag)
    pressed = false
    dragged = false
    origin = nil
    swallow_press = true
    if not was_drag then
        activate()
    end
end

local function on_left(event)
    local state = event and event.event or "press"
    if event and event.canceled then
        -- A drag already consumed this click. Drop a late mouse-up too.
        pressed = false
        dragged = false
        origin = nil
        swallow_press = true
        return
    end
    if state == "down" then
        pressed = true
        dragged = false
        swallow_press = false
        local x, y = pointer()
        origin = { x = x, y = y, edge = near_edge(x, y) }
        return
    end
    if state == "up" then
        -- performDrag can deliver a second mouse-up after the gesture ended.
        if not pressed then
            return
        end
        finish_gesture(dragged)
        return
    end
    if state == "press" and not swallow_press and not pressed and not dragged then
        activate()
    end
    swallow_press = false
    pressed = false
    dragged = false
    origin = nil
end

mp.add_forced_key_binding("MBTN_LEFT", "pinplay-left", on_left, { complex = true })
mp.add_forced_key_binding("MBTN_LEFT_DBL", "pinplay-left-double", function() end)
mp.add_forced_key_binding("MBTN_RIGHT", "pinplay-right", toggle_menu)
mp.add_forced_key_binding("MOUSE_MOVE", "pinplay-move", function()
    if menu then
        render()
        return
    end
    track_drag()
end)
mp.observe_property("pause", "bool", function()
    if menu then
        render()
    end
end)
mp.observe_property("mute", "bool", function()
    if menu then
        render()
    end
end)
mp.observe_property("osd-width", "number", function()
    if pressed then
        dragged = true
    end
    if menu == "size" then
        render()
    end
end)
mp.observe_property("osd-height", "number", function()
    if pressed then
        dragged = true
    end
end)
