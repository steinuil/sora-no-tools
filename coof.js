var ONE_FRAME = 1 / 60;

var last_time;

var overlay = mp.create_osd_overlay("ass-events");

var comments = [];

var CHAR_WIDTH = 50;
var LINE_HEIGHT = 50;

var IRC_COLOR_TO_HEX_BGR = [
  "FFFFFF", // white
  "000000", // black
  "7F0000", // blue
  "009300", // green
  "0000FF", // red
  "00007F", // brown
  "9C009C", // magenta
  "007FFC", // orange
  "00FFFF", // yellow
  "00FC00", // light green
  "939300", // cyan
  "FFFF00", // light cyan
  "FC0000", // light blue
  "FF00FF", // pink
  "7F7F7F", // grey
  "D2D2D2", // light grey
  // https://modern.ircdocs.horse/formatting.html#colors-16-98 for more
  // "000047",
  // "002147",
  // "004747",
  // "004723",
  // "004700",
  // "00472C",
];

function message_length(msg) {
  return msg.length * CHAR_WIDTH;
}

function irc_color_to_hex_bgr(color) {
  if (!color) return;
  var n = parseInt(color, 10);
  if (isNaN(n)) return;
  return IRC_COLOR_TO_HEX_BGR[n];
}

function irc_formatting_to_ass_tags(msg) {
  var out = "";

  for (var i = 0; i < msg.length; i++) {
    switch (msg.charCodeAt(i)) {
      case 0x02: // bold
        out = out + "{\\b1}";
        break;
      case 0x1d: // italic
        out = out + "{\\i1}";
        break;
      case 0x1f: // underline
        out = out + "{\\u1}";
        break;
      case 0x1e: // strikethrough
        out = out + "{\\s1}";
        break;
      case 0x03: // color
        {
          var colors = msg.substring(i + 1).match(/^(\d\d?)(,(\d\d?))?/);
          if (!colors) break;
          i += colors[0].length;
          var fg = irc_color_to_hex_bgr(colors[1]);
          var bg = irc_color_to_hex_bgr(colors[3]);

          out =
            out +
            (fg ? "{\\1c&H" + fg + "&}" : "") +
            (bg ? "{\\3c&H" + bg + "&}" : "");
        }
        break;
      case 0x0f: // reset
        out = out + "{\\r}";
        break;
      case 0x11: // monospace
      case 0x04: // hex color
      case 0x16: // reverse color
        // ignore
        break;
      default:
        out = out + msg[i];
    }
  }

  return out;
}

function danmaku_lines() {
  return mp.get_property("dheight") / LINE_HEIGHT;
}

function minIndex(arr) {
  var min = 0;

  for (var i = 1; i < arr.length; i++) {
    if (arr[i] < arr[min]) {
      min = i;
    }
  }

  return min;
}

function find_available_y() {
  var lines = danmaku_lines();

  var lineCountsByY = [];

  for (var i = 0; i < lines; i++) {
    lineCountsByY[i] = comments.filter(function (comment) {
      return comment.y === i;
    }).length;
  }

  return minIndex(lineCountsByY);
}

function update_danmaku() {
  var current_time = mp.get_time();
  var frame_time =
    last_time === undefined ? ONE_FRAME : current_time - last_time;

  last_time = current_time;

  overlay.data = comments
    .map(function (comment) {
      return (
        "{\\pos(" +
        comment.x.toString() +
        ", " +
        (comment.y * LINE_HEIGHT).toString() +
        ")}" +
        comment.text
      );
    })
    .join("\n");

  overlay.update();

  comments = comments.filter(function (comment) {
    return comment.x > -message_length(comment.text);
  });

  comments.forEach(function (comment) {
    // Speed is the number of pixels the text moves in one frame.
    comment.x = comment.x - comment.speed * (frame_time / ONE_FRAME);
  });
}

var danmaku_timer;

mp.register_event("video-reconfig", function () {
  if (danmaku_timer !== undefined) return;
  danmaku_timer = setInterval(update_danmaku, 1000 / 60);
});

mp.register_script_message("danmaku-message", function (msg) {
  // TODO decide the speed based on the length of the line

  comments.push({
    text: irc_formatting_to_ass_tags(msg),
    x: mp.get_property("dwidth"),
    y: find_available_y(),
    speed: 8,
  });
});
