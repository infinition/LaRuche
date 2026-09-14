/* SVG chart renderer for notebook cell outputs.
 *
 * Emits a standalone SVG string with explicit colours, so the same markup is
 * what the viewer sees and what the PNG export rasterises. Charts are
 * re-rendered when the host theme changes rather than relying on CSS
 * variables, which would not survive the export.
 *
 * Palette: the reference categorical order, validated with the data-viz
 * validator against both surfaces used here (worst adjacent CVD dE 8.4 dark /
 * 9.1 light, normal-vision 19.3 / 19.6). Slot order is the CVD-safety
 * mechanism, so hues are assigned in order and never cycled: past eight
 * series the caller folds the tail into "Other".
 */
(function(root, factory){
  'use strict';
  var api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioChart = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(){
  'use strict';

  var PALETTE = {
    dark: ['#3987e5', '#d95926', '#199e70', '#c98500', '#d55181', '#008300', '#9085e9', '#e66767'],
    light: ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#4a3aa7', '#e34948']
  };

  var THEMES = {
    dark: { surface: '#191922', grid: '#2f2f3b', axis: '#3a3a48', text: '#f7f4ed', muted: '#9b98a5' },
    light: { surface: '#ffffff', grid: '#e6e5e1', axis: '#cfcec9', text: '#1a1a19', muted: '#66655f' }
  };

  var MAX_SERIES = 8;
  var BAR_MAX = 24;
  var GAP = 2;

  /* Separators for the crosshair tooltip payload, spelled without escapes. */
  var FIELD_SEPARATOR = String.fromCharCode(31);
  var POINT_SEPARATOR = String.fromCharCode(30);

  function escapeXml(value) {
    return String(value === null || value === undefined ? '' : value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function defaultNumber(value) {
    if (value === null || value === undefined || isNaN(value)) return '';
    var abs = Math.abs(value);
    if (abs >= 1e9) return (value / 1e9).toFixed(1).replace(/\.0$/, '') + 'G';
    if (abs >= 1e6) return (value / 1e6).toFixed(1).replace(/\.0$/, '') + 'M';
    if (abs >= 1e4) return Math.round(value / 1e3) + 'k';
    if (Number.isInteger(value)) return String(value);
    if (abs >= 100) return value.toFixed(0);
    if (abs >= 1) return value.toFixed(2).replace(/0$/, '').replace(/\.$/, '');
    return value.toPrecision(3).replace(/0+$/, '').replace(/\.$/, '');
  }

  /* Axis ticks land on 1, 2, 2.5 or 5 times a power of ten. */
  function niceTicks(min, max, count) {
    var target = count || 5;
    if (min === max) {
      if (min === 0) return { min: 0, max: 1, step: 0.25, ticks: [0, 0.25, 0.5, 0.75, 1] };
      var pad = Math.abs(min) * 0.5;
      min -= pad;
      max += pad;
    }
    var span = max - min;
    var rough = span / target;
    var magnitude = Math.pow(10, Math.floor(Math.log(rough) / Math.LN10));
    var normalised = rough / magnitude;
    var step = magnitude * (normalised <= 1 ? 1 : normalised <= 2 ? 2 : normalised <= 2.5 ? 2.5 : normalised <= 5 ? 5 : 10);
    var lower = Math.floor(min / step) * step;
    var upper = Math.ceil(max / step) * step;
    var ticks = [];
    var decimals = Math.max(0, -Math.floor(Math.log(step) / Math.LN10));
    for (var value = lower; value <= upper + step * 0.5; value += step) {
      ticks.push(Number(value.toFixed(decimals + 2)));
    }
    return { min: lower, max: upper, step: step, ticks: ticks };
  }

  function seriesExtent(series, stacked) {
    var min = 0;
    var max = 0;
    if (stacked) {
      var length = series.length ? series[0].values.length : 0;
      for (var i = 0; i < length; i += 1) {
        var positive = 0;
        var negative = 0;
        for (var s = 0; s < series.length; s += 1) {
          var value = series[s].values[i];
          if (typeof value !== 'number' || isNaN(value)) continue;
          if (value >= 0) positive += value; else negative += value;
        }
        if (positive > max) max = positive;
        if (negative < min) min = negative;
      }
      return { min: min, max: max };
    }
    series.forEach(function(entry){
      entry.values.forEach(function(value){
        if (typeof value !== 'number' || isNaN(value)) return;
        if (value > max) max = value;
        if (value < min) min = value;
      });
    });
    return { min: min, max: max };
  }

  /* Keeps the fixed hue order and folds anything past slot eight into a single
   * "Other" series, rather than inventing a ninth colour. */
  function capSeries(series, otherLabel) {
    if (series.length <= MAX_SERIES) return series;
    var kept = series.slice(0, MAX_SERIES - 1);
    var rest = series.slice(MAX_SERIES - 1);
    var length = series[0].values.length;
    var merged = new Array(length);
    for (var i = 0; i < length; i += 1) {
      var total = null;
      for (var s = 0; s < rest.length; s += 1) {
        var value = rest[s].values[i];
        if (typeof value === 'number' && !isNaN(value)) total = (total || 0) + value;
      }
      merged[i] = total;
    }
    kept.push({ name: otherLabel || 'Other', values: merged, folded: rest.length });
    return kept;
  }

  function truncate(text, limit) {
    var value = String(text);
    return value.length > limit ? value.slice(0, limit - 1) + '…' : value;
  }

  /* Rough advance width for the label sizing pass. The renderer only needs to
   * know whether a label fits, not to typeset it. */
  function textWidth(text, size) {
    return String(text).length * size * 0.56;
  }

  function Builder() {
    this.parts = [];
  }

  Builder.prototype.add = function(markup) {
    this.parts.push(markup);
    return this;
  };

  Builder.prototype.text = function(x, y, content, options) {
    var settings = options || {};
    this.parts.push(
      '<text x="' + round(x) + '" y="' + round(y) + '"' +
      ' fill="' + (settings.fill || '#000') + '"' +
      ' font-size="' + (settings.size || 11) + '"' +
      ' font-weight="' + (settings.weight || 400) + '"' +
      ' text-anchor="' + (settings.anchor || 'start') + '"' +
      (settings.transform ? ' transform="' + settings.transform + '"' : '') +
      ' font-family="Inter, ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif"' +
      '>' + escapeXml(content) + '</text>'
    );
    return this;
  };

  function round(value) {
    return Math.round(value * 100) / 100;
  }

  /* --------------------------------------------------------------- Layout */

  function measure(spec, theme, options) {
    var settings = options || {};
    var width = settings.width || 720;
    var labelSize = 11;
    var series = capSeries(spec.series || [], settings.otherLabel);
    var horizontal = !!spec.horizontal && spec.chart === 'bar';

    var longestLabel = 0;
    (spec.labels || []).forEach(function(label){
      longestLabel = Math.max(longestLabel, textWidth(truncate(label, 22), labelSize));
    });

    var extent = seriesExtent(series, spec.stacked);
    var scale = niceTicks(extent.min, extent.max, 5);
    var longestTick = 0;
    scale.ticks.forEach(function(tick){
      longestTick = Math.max(longestTick, textWidth((settings.formatNumber || defaultNumber)(tick), labelSize));
    });

    var rotate = false;
    var count = (spec.labels || []).length || 1;
    if (!horizontal) {
      var available = (width - 60 - longestTick) / count;
      rotate = longestLabel > available;
    }

    var margin = {
      top: spec.title ? 34 : 16,
      right: 18,
      bottom: horizontal ? 44 : (rotate ? 20 + Math.min(longestLabel, 90) : 42),
      left: horizontal ? Math.min(longestLabel + 16, width * 0.42) : longestTick + 20
    };

    if (series.length > 1) margin.bottom += 26;

    var height = settings.height || Math.max(240, Math.min(460, margin.top + margin.bottom + count * (horizontal ? 26 : 0) + 180));

    return {
      width: width,
      height: height,
      margin: margin,
      series: series,
      scale: scale,
      rotate: rotate,
      horizontal: horizontal,
      labelSize: labelSize,
      plot: {
        x: margin.left,
        y: margin.top,
        width: Math.max(40, width - margin.left - margin.right),
        height: Math.max(60, height - margin.top - margin.bottom)
      }
    };
  }

  function drawFrame(builder, spec, layout, theme, palette, formatNumber) {
    var plot = layout.plot;

    if (spec.title) {
      builder.text(layout.margin.left, 20, spec.title, { fill: theme.text, size: 13, weight: 700 });
    }

    var ticks = layout.scale.ticks;
    var span = layout.scale.max - layout.scale.min || 1;

    if (layout.horizontal) {
      ticks.forEach(function(tick){
        var x = plot.x + ((tick - layout.scale.min) / span) * plot.width;
        builder.add('<line x1="' + round(x) + '" y1="' + plot.y + '" x2="' + round(x) + '" y2="' + round(plot.y + plot.height) + '" stroke="' + theme.grid + '" stroke-width="1"/>');
        builder.text(x, plot.y + plot.height + 16, formatNumber(tick), { fill: theme.muted, size: layout.labelSize, anchor: 'middle' });
      });
    } else {
      ticks.forEach(function(tick){
        var y = plot.y + plot.height - ((tick - layout.scale.min) / span) * plot.height;
        builder.add('<line x1="' + plot.x + '" y1="' + round(y) + '" x2="' + round(plot.x + plot.width) + '" y2="' + round(y) + '" stroke="' + theme.grid + '" stroke-width="1"/>');
        builder.text(plot.x - 8, round(y + 4), formatNumber(tick), { fill: theme.muted, size: layout.labelSize, anchor: 'end' });
      });
    }

    var zero = layout.scale.min < 0 && layout.scale.max > 0;
    if (zero) {
      if (layout.horizontal) {
        var zeroX = plot.x + ((0 - layout.scale.min) / span) * plot.width;
        builder.add('<line x1="' + round(zeroX) + '" y1="' + plot.y + '" x2="' + round(zeroX) + '" y2="' + round(plot.y + plot.height) + '" stroke="' + theme.axis + '" stroke-width="1"/>');
      } else {
        var zeroY = plot.y + plot.height - ((0 - layout.scale.min) / span) * plot.height;
        builder.add('<line x1="' + plot.x + '" y1="' + round(zeroY) + '" x2="' + round(plot.x + plot.width) + '" y2="' + round(zeroY) + '" stroke="' + theme.axis + '" stroke-width="1"/>');
      }
    }
  }

  function drawLegend(builder, layout, theme, palette) {
    if (layout.series.length < 2) return;
    var y = layout.height - 12;
    var x = layout.margin.left;
    layout.series.forEach(function(entry, index){
      var label = truncate(entry.name, 18);
      var width = 14 + textWidth(label, 11) + 14;
      if (x + width > layout.width - 8 && index > 0) return;
      builder.add('<rect x="' + round(x) + '" y="' + round(y - 8) + '" width="9" height="9" rx="2" fill="' + palette[index % palette.length] + '"/>');
      builder.text(x + 14, y, label, { fill: theme.muted, size: 11 });
      x += width;
    });
  }

  function valueToY(value, layout) {
    var span = layout.scale.max - layout.scale.min || 1;
    return layout.plot.y + layout.plot.height - ((value - layout.scale.min) / span) * layout.plot.height;
  }

  function valueToX(value, layout) {
    var span = layout.scale.max - layout.scale.min || 1;
    return layout.plot.x + ((value - layout.scale.min) / span) * layout.plot.width;
  }

  function markAttributes(label, name, value, formatNumber) {
    return ' data-label="' + escapeXml(label) + '" data-series="' + escapeXml(name) +
      '" data-value="' + escapeXml(formatNumber(value)) + '"';
  }

  /* ----------------------------------------------------------- Bar chart */

  function renderBars(builder, spec, layout, theme, palette, formatNumber) {
    var plot = layout.plot;
    var labels = spec.labels || [];
    var count = labels.length || 1;
    var stacked = !!spec.stacked;
    var groups = stacked ? 1 : layout.series.length;

    var band = (layout.horizontal ? plot.height : plot.width) / count;
    var usable = band * 0.72;
    var thickness = Math.max(3, Math.min(BAR_MAX, (usable - (groups - 1) * GAP) / groups));
    var groupWidth = thickness * groups + GAP * (groups - 1);

    labels.forEach(function(label, index){
      var start = (layout.horizontal ? plot.y : plot.x) + band * index + (band - groupWidth) / 2;
      var positive = 0;
      var negative = 0;

      layout.series.forEach(function(entry, seriesIndex){
        var value = entry.values[index];
        if (typeof value !== 'number' || isNaN(value)) return;
        var colour = palette[seriesIndex % palette.length];
        var attributes = markAttributes(label, entry.name, value, formatNumber);

        if (layout.horizontal) {
          var baseX = valueToX(stacked ? (value >= 0 ? positive : negative) : 0, layout);
          var endX = valueToX(stacked ? (value >= 0 ? positive + value : negative + value) : value, layout);
          var top = stacked ? start : start + seriesIndex * (thickness + GAP);
          var barHeight = stacked ? groupWidth : thickness;
          var left = Math.min(baseX, endX);
          var barWidth = Math.max(1, Math.abs(endX - baseX) - (stacked ? GAP : 0));
          builder.add(
            '<path class="mark"' + attributes + ' fill="' + colour + '" d="' +
            roundedBar(left, top, barWidth, barHeight, value >= 0 ? 'right' : 'left') + '"/>'
          );
          if (value >= 0) positive += value; else negative += value;
        } else {
          var baseY = valueToY(stacked ? (value >= 0 ? positive : negative) : 0, layout);
          var endY = valueToY(stacked ? (value >= 0 ? positive + value : negative + value) : value, layout);
          var left2 = stacked ? start : start + seriesIndex * (thickness + GAP);
          var barWidth2 = stacked ? groupWidth : thickness;
          var top2 = Math.min(baseY, endY);
          var barHeight2 = Math.max(1, Math.abs(endY - baseY) - (stacked ? GAP : 0));
          builder.add(
            '<path class="mark"' + attributes + ' fill="' + colour + '" d="' +
            roundedBar(left2, top2, barWidth2, barHeight2, value >= 0 ? 'top' : 'bottom') + '"/>'
          );
          if (value >= 0) positive += value; else negative += value;
        }
      });

      /* Direct value label on a single series only, and only where it fits. */
      if (layout.series.length === 1 && !stacked) {
        var only = layout.series[0].values[index];
        if (typeof only === 'number' && !isNaN(only)) {
          var text = formatNumber(only);
          if (layout.horizontal) {
            var tipX = valueToX(only, layout);
            if (tipX + 6 + textWidth(text, 10) < plot.x + plot.width) {
              builder.text(tipX + 6, start + groupWidth / 2 + 3.5, text, { fill: theme.muted, size: 10 });
            }
          } else {
            var tipY = valueToY(only, layout);
            if (tipY - 6 > plot.y) {
              builder.text(start + groupWidth / 2, tipY - 6, text, { fill: theme.muted, size: 10, anchor: 'middle' });
            }
          }
        }
      }

      var labelText = truncate(label, layout.horizontal ? 26 : 18);
      if (layout.horizontal) {
        builder.text(plot.x - 8, start + groupWidth / 2 + 3.5, labelText, { fill: theme.muted, size: layout.labelSize, anchor: 'end' });
      } else if (layout.rotate) {
        var cx = plot.x + band * index + band / 2;
        var cy = plot.y + plot.height + 12;
        builder.text(cx, cy, labelText, {
          fill: theme.muted,
          size: layout.labelSize,
          anchor: 'end',
          transform: 'rotate(-45 ' + round(cx) + ' ' + round(cy) + ')'
        });
      } else {
        builder.text(plot.x + band * index + band / 2, plot.y + plot.height + 16, labelText, {
          fill: theme.muted, size: layout.labelSize, anchor: 'middle'
        });
      }
    });
  }

  /* 4px rounded data-end, square at the baseline. */
  function roundedBar(x, y, width, height, side) {
    var radius = Math.min(4, width / 2, height / 2);
    if (radius <= 0.5) {
      return 'M' + round(x) + ' ' + round(y) + 'h' + round(width) + 'v' + round(height) + 'h' + round(-width) + 'Z';
    }
    if (side === 'top') {
      return 'M' + round(x) + ' ' + round(y + height) +
        'V' + round(y + radius) + 'a' + radius + ' ' + radius + ' 0 0 1 ' + radius + ' ' + -radius +
        'h' + round(width - radius * 2) +
        'a' + radius + ' ' + radius + ' 0 0 1 ' + radius + ' ' + radius +
        'V' + round(y + height) + 'Z';
    }
    if (side === 'bottom') {
      return 'M' + round(x) + ' ' + round(y) +
        'V' + round(y + height - radius) +
        'a' + radius + ' ' + radius + ' 0 0 0 ' + radius + ' ' + radius +
        'h' + round(width - radius * 2) +
        'a' + radius + ' ' + radius + ' 0 0 0 ' + radius + ' ' + -radius +
        'V' + round(y) + 'Z';
    }
    if (side === 'right') {
      return 'M' + round(x) + ' ' + round(y) +
        'h' + round(width - radius) +
        'a' + radius + ' ' + radius + ' 0 0 1 ' + radius + ' ' + radius +
        'v' + round(height - radius * 2) +
        'a' + radius + ' ' + radius + ' 0 0 1 ' + -radius + ' ' + radius +
        'h' + round(-(width - radius)) + 'Z';
    }
    return 'M' + round(x + width) + ' ' + round(y) +
      'h' + round(-(width - radius)) +
      'a' + radius + ' ' + radius + ' 0 0 0 ' + -radius + ' ' + radius +
      'v' + round(height - radius * 2) +
      'a' + radius + ' ' + radius + ' 0 0 0 ' + radius + ' ' + radius +
      'h' + round(width - radius) + 'Z';
  }

  /* ---------------------------------------------------- Line and area */

  function renderLines(builder, spec, layout, theme, palette, formatNumber, filled) {
    var plot = layout.plot;
    var labels = spec.labels || [];
    var count = labels.length;
    var step = count > 1 ? plot.width / (count - 1) : 0;
    var pointX = function(index){ return count > 1 ? plot.x + step * index : plot.x + plot.width / 2; };

    layout.series.forEach(function(entry, seriesIndex){
      var colour = palette[seriesIndex % palette.length];
      var segments = [];
      var current = [];
      entry.values.forEach(function(value, index){
        if (typeof value !== 'number' || isNaN(value)) {
          if (current.length) segments.push(current);
          current = [];
          return;
        }
        current.push({ x: pointX(index), y: valueToY(value, layout), value: value, index: index });
      });
      if (current.length) segments.push(current);

      segments.forEach(function(points){
        if (filled && points.length > 1) {
          var baseline = valueToY(Math.max(layout.scale.min, 0), layout);
          var area = 'M' + round(points[0].x) + ' ' + round(baseline) +
            points.map(function(point){ return 'L' + round(point.x) + ' ' + round(point.y); }).join('') +
            'L' + round(points[points.length - 1].x) + ' ' + round(baseline) + 'Z';
          builder.add('<path d="' + area + '" fill="' + colour + '" fill-opacity="0.1"/>');
        }
        if (points.length === 1) {
          builder.add('<circle cx="' + round(points[0].x) + '" cy="' + round(points[0].y) + '" r="4" fill="' + colour + '" stroke="' + theme.surface + '" stroke-width="2"/>');
          return;
        }
        var path = 'M' + points.map(function(point, position){
          return (position ? 'L' : '') + round(point.x) + ' ' + round(point.y);
        }).join('');
        builder.add('<path d="' + path + '" fill="none" stroke="' + colour + '" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>');
      });

      /* End marker with a surface ring, plus the endpoint value. */
      var last = null;
      for (var i = entry.values.length - 1; i >= 0; i -= 1) {
        if (typeof entry.values[i] === 'number' && !isNaN(entry.values[i])) {
          last = { x: pointX(i), y: valueToY(entry.values[i], layout), value: entry.values[i] };
          break;
        }
      }
      if (last) {
        builder.add('<circle cx="' + round(last.x) + '" cy="' + round(last.y) + '" r="4" fill="' + colour + '" stroke="' + theme.surface + '" stroke-width="2"/>');
        if (layout.series.length <= 4) {
          var text = formatNumber(last.value);
          if (last.x + 8 + textWidth(text, 10) < layout.width - 4) {
            builder.text(last.x + 8, last.y + 3.5, text, { fill: theme.muted, size: 10 });
          }
        }
      }
    });

    /* Invisible hover columns: one per category, wider than the marks. */
    labels.forEach(function(label, index){
      var x = pointX(index);
      var half = count > 1 ? step / 2 : plot.width / 2;
      var values = layout.series.map(function(entry){
        return { name: entry.name, value: entry.values[index] };
      }).filter(function(entry){ return typeof entry.value === 'number' && !isNaN(entry.value); });
      if (!values.length) return;
      builder.add(
        '<rect class="mark crosshair" x="' + round(x - half) + '" y="' + plot.y + '" width="' + round(half * 2) + '" height="' + round(plot.height) + '"' +
        ' fill="transparent" data-label="' + escapeXml(label) + '" data-x="' + round(x) + '"' +
        ' data-points="' + escapeXml(values.map(function(entry){ return entry.name + FIELD_SEPARATOR + formatNumber(entry.value); }).join(POINT_SEPARATOR)) + '"/>'
      );
    });

    var tickEvery = Math.max(1, Math.ceil(count / Math.max(2, Math.floor(plot.width / 70))));
    labels.forEach(function(label, index){
      if (index % tickEvery !== 0 && index !== count - 1) return;
      var x = pointX(index);
      var text = truncate(label, 14);
      if (layout.rotate) {
        var cy = plot.y + plot.height + 12;
        builder.text(x, cy, text, {
          fill: theme.muted, size: layout.labelSize, anchor: 'end',
          transform: 'rotate(-45 ' + round(x) + ' ' + round(cy) + ')'
        });
      } else {
        builder.text(x, plot.y + plot.height + 16, text, { fill: theme.muted, size: layout.labelSize, anchor: 'middle' });
      }
    });
  }

  /* ------------------------------------------------------------ Scatter */

  function renderScatter(builder, spec, layout, theme, palette, formatNumber) {
    var plot = layout.plot;
    var labels = spec.labels || [];
    var numericLabels = labels.map(function(label){
      var value = Number(label);
      return isNaN(value) ? null : value;
    });
    var categorical = numericLabels.some(function(value){ return value === null; });

    var xScale;
    if (categorical) {
      xScale = function(index){
        return plot.x + (labels.length > 1 ? (plot.width / (labels.length - 1)) * index : plot.width / 2);
      };
    } else {
      var xTicks = niceTicks(Math.min.apply(null, numericLabels), Math.max.apply(null, numericLabels), 5);
      var xSpan = xTicks.max - xTicks.min || 1;
      xScale = function(index){
        return plot.x + ((numericLabels[index] - xTicks.min) / xSpan) * plot.width;
      };
      xTicks.ticks.forEach(function(tick){
        var x = plot.x + ((tick - xTicks.min) / xSpan) * plot.width;
        builder.text(x, plot.y + plot.height + 16, formatNumber(tick), { fill: theme.muted, size: layout.labelSize, anchor: 'middle' });
      });
    }

    layout.series.forEach(function(entry, seriesIndex){
      var colour = palette[seriesIndex % palette.length];
      entry.values.forEach(function(value, index){
        if (typeof value !== 'number' || isNaN(value)) return;
        builder.add(
          '<circle class="mark" cx="' + round(xScale(index)) + '" cy="' + round(valueToY(value, layout)) + '" r="4.5"' +
          ' fill="' + colour + '" stroke="' + theme.surface + '" stroke-width="2"' +
          markAttributes(labels[index], entry.name, value, formatNumber) + '/>'
        );
      });
    });

    if (categorical) {
      var tickEvery = Math.max(1, Math.ceil(labels.length / Math.max(2, Math.floor(plot.width / 70))));
      labels.forEach(function(label, index){
        if (index % tickEvery !== 0) return;
        builder.text(xScale(index), plot.y + plot.height + 16, truncate(label, 12), {
          fill: theme.muted, size: layout.labelSize, anchor: 'middle'
        });
      });
    }
  }

  /* ---------------------------------------------------------------- Pie */

  function renderPie(spec, theme, palette, formatNumber, options) {
    var settings = options || {};
    var width = settings.width || 720;
    var labels = spec.labels || [];
    var values = (spec.series[0] || { values: [] }).values;

    var entries = labels.map(function(label, index){
      return { label: label, value: typeof values[index] === 'number' && values[index] > 0 ? values[index] : 0 };
    }).filter(function(entry){ return entry.value > 0; });

    var total = entries.reduce(function(sum, entry){ return sum + entry.value; }, 0);
    if (!total) return null;

    if (entries.length > MAX_SERIES) {
      var kept = entries.slice(0, MAX_SERIES - 1);
      var rest = entries.slice(MAX_SERIES - 1);
      kept.push({
        label: settings.otherLabel || 'Other',
        value: rest.reduce(function(sum, entry){ return sum + entry.value; }, 0),
        folded: rest.length
      });
      entries = kept;
    }

    var legendRows = entries.length;
    var height = Math.max(240, Math.min(420, 60 + legendRows * 20));
    var radius = Math.min(height * 0.38, width * 0.22);
    var centreX = radius + 30;
    var centreY = height / 2 + (spec.title ? 8 : 0);

    var builder = new Builder();
    builder.add('<rect width="' + width + '" height="' + height + '" fill="' + theme.surface + '"/>');
    if (spec.title) {
      builder.text(16, 20, spec.title, { fill: theme.text, size: 13, weight: 700 });
    }

    var angle = -Math.PI / 2;
    entries.forEach(function(entry, index){
      var slice = (entry.value / total) * Math.PI * 2;
      var gapAngle = Math.min(slice * 0.06, GAP / radius);
      var start = angle + gapAngle / 2;
      var end = angle + slice - gapAngle / 2;
      var inner = radius * 0.58;
      var large = slice > Math.PI ? 1 : 0;

      var path = 'M' + round(centreX + Math.cos(start) * radius) + ' ' + round(centreY + Math.sin(start) * radius) +
        'A' + round(radius) + ' ' + round(radius) + ' 0 ' + large + ' 1 ' +
        round(centreX + Math.cos(end) * radius) + ' ' + round(centreY + Math.sin(end) * radius) +
        'L' + round(centreX + Math.cos(end) * inner) + ' ' + round(centreY + Math.sin(end) * inner) +
        'A' + round(inner) + ' ' + round(inner) + ' 0 ' + large + ' 0 ' +
        round(centreX + Math.cos(start) * inner) + ' ' + round(centreY + Math.sin(start) * inner) + 'Z';

      var share = (entry.value / total) * 100;
      builder.add(
        '<path class="mark" d="' + path + '" fill="' + palette[index % palette.length] + '"' +
        markAttributes(entry.label, formatNumber(entry.value), share, function(v){ return v.toFixed(1) + ' %'; }) + '/>'
      );
      angle += slice;
    });

    builder.text(centreX, centreY + 4, formatNumber(total), { fill: theme.text, size: 15, weight: 700, anchor: 'middle' });

    var legendX = centreX + radius + 24;
    var legendTop = centreY - (legendRows * 20) / 2 + 10;
    entries.forEach(function(entry, index){
      var y = legendTop + index * 20;
      builder.add('<rect x="' + round(legendX) + '" y="' + round(y - 8) + '" width="9" height="9" rx="2" fill="' + palette[index % palette.length] + '"/>');
      var share = ((entry.value / total) * 100).toFixed(1) + ' %';
      var room = width - legendX - 14 - textWidth(share, 11) - 12;
      builder.text(legendX + 14, y, truncate(entry.label, Math.max(6, Math.floor(room / 6.2))), { fill: theme.muted, size: 11 });
      builder.text(width - 12, y, share, { fill: theme.text, size: 11, anchor: 'end' });
    });

    return { svg: wrap(builder, width, height, spec.title), width: width, height: height };
  }

  /* Une couleur fournie par l'appelant finit dans un attribut fill du SVG.
   * Seul du parfaitement hexadecimal passe, donc rien d'arbitraire n'y arrive.
   * Le langage studio refusait deja le reste en amont, mais un noyau Python
   * ecrit du Python ordinaire et n'a pas ce filtre: la garde a sa place ici,
   * au seul endroit par ou toutes les couleurs passent. */
  var HEX = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

  function withOverrides(base, colors) {
    if (!colors || !colors.length) return base;
    var merged = base.slice();
    for (var i = 0; i < colors.length && i < merged.length; i += 1) {
      if (colors[i] && HEX.test(String(colors[i]).trim())) merged[i] = String(colors[i]).trim();
    }
    return merged;
  }

  /* --------------------------------------------------------- 3D scatter */

  /* Three measures on three axes, projected by hand: no library is reachable
   * under the App content policy, and none is needed for a point cloud.
   *
   * A caveat worth stating, because the renderer cannot state it for you: on a
   * flat screen a third axis costs accuracy. Depth is read from occlusion and
   * from the box, both of which are weak cues, so comparing two points along
   * the depth axis is guesswork. Use it to look for shape and clustering, and
   * a plain 2D scatter or a small-multiple grid when a value has to be read.
   *
   * `spec.points` is [{x, y, z, series, size, label}]. Rotation is the
   * caller's business: it re-renders with new yaw and pitch. */
  function renderScatter3d(spec, theme, palette, formatNumber, options) {
    var settings = options || {};
    var width = settings.width || 720;
    var height = settings.height || Math.round(width * 0.62);
    var points = spec.points || [];
    if (!points.length) return null;

    var yaw = settings.yaw === undefined ? -0.62 : settings.yaw;
    var pitch = settings.pitch === undefined ? 0.42 : settings.pitch;

    var axes = ['x', 'y', 'z'];
    var extent = {};
    axes.forEach(function(axis){
      var min = Infinity;
      var max = -Infinity;
      points.forEach(function(point){
        var value = point[axis];
        if (typeof value !== 'number' || !isFinite(value)) return;
        if (value < min) min = value;
        if (value > max) max = value;
      });
      if (!isFinite(min)) { min = 0; max = 1; }
      if (min === max) { max = min + 1; }
      extent[axis] = { min: min, max: max, span: max - min };
    });

    function normalise(value, axis) {
      return (value - extent[axis].min) / extent[axis].span - 0.5;
    }

    var cosYaw = Math.cos(yaw);
    var sinYaw = Math.sin(yaw);
    var cosPitch = Math.cos(pitch);
    var sinPitch = Math.sin(pitch);
    var distance = 3.2;

    var centreX = width / 2;
    var centreY = height / 2 + (spec.title ? 8 : 0);
    var radius = Math.min(width, height) * 0.46;

    /* Yaw about the vertical axis, then pitch, then a gentle perspective so
     * the near face of the box reads as nearer. */
    function project(px, py, pz) {
      var x1 = px * cosYaw + pz * sinYaw;
      var z1 = pz * cosYaw - px * sinYaw;
      var y2 = py * cosPitch - z1 * sinPitch;
      var z2 = py * sinPitch + z1 * cosPitch;
      var scale = distance / (distance + z2);
      return {
        x: centreX + x1 * scale * radius,
        y: centreY - y2 * scale * radius,
        depth: z2,
        scale: scale
      };
    }

    var builder = new Builder();
    builder.add('<rect width="' + width + '" height="' + height + '" fill="' + theme.surface + '"/>');
    if (spec.title) {
      builder.text(16, 20, spec.title, { fill: theme.text, size: 13, weight: 700 });
    }

    /* The unit cube: eight corners, twelve edges. Edges behind the cloud are
     * drawn first and fainter, so the box reads as a box. */
    var corners = [];
    for (var c = 0; c < 8; c += 1) {
      corners.push(project(
        (c & 1 ? 0.5 : -0.5),
        (c & 2 ? 0.5 : -0.5),
        (c & 4 ? 0.5 : -0.5)
      ));
    }
    var EDGES = [
      [0, 1], [2, 3], [4, 5], [6, 7],
      [0, 2], [1, 3], [4, 6], [5, 7],
      [0, 4], [1, 5], [2, 6], [3, 7]
    ];
    var edges = EDGES.map(function(edge){
      return {
        a: corners[edge[0]],
        b: corners[edge[1]],
        depth: (corners[edge[0]].depth + corners[edge[1]].depth) / 2
      };
    }).sort(function(left, right){ return right.depth - left.depth; });

    edges.forEach(function(edge, index){
      builder.add(
        '<line x1="' + round(edge.a.x) + '" y1="' + round(edge.a.y) +
        '" x2="' + round(edge.b.x) + '" y2="' + round(edge.b.y) +
        '" stroke="' + theme.grid + '" stroke-width="1" stroke-opacity="' +
        (index < 6 ? '0.5' : '1') + '"/>'
      );
    });

    /* Axis names at the middle of the three edges meeting the near-bottom
     * corner, plus the range at each end. */
    var labels = [
      { name: (spec.axis && spec.axis.x) || 'x', from: [-0.5, -0.5, -0.5], to: [0.5, -0.5, -0.5], axis: 'x' },
      { name: (spec.axis && spec.axis.y) || 'y', from: [-0.5, -0.5, -0.5], to: [-0.5, 0.5, -0.5], axis: 'y' },
      { name: (spec.axis && spec.axis.z) || 'z', from: [-0.5, -0.5, -0.5], to: [-0.5, -0.5, 0.5], axis: 'z' }
    ];
    var origin = project(-0.5, -0.5, -0.5);

    /* The three axes share one corner, so their minimum labels would land on
     * top of each other there. Each is moved a short way along its own axis,
     * and the names are pushed outward from the box so they clear the cloud. */
    labels.forEach(function(entry){
      var start = project(entry.from[0], entry.from[1], entry.from[2]);
      var end = project(entry.to[0], entry.to[1], entry.to[2]);
      builder.add(
        '<line x1="' + round(start.x) + '" y1="' + round(start.y) +
        '" x2="' + round(end.x) + '" y2="' + round(end.y) +
        '" stroke="' + theme.axis + '" stroke-width="1.5"/>'
      );

      function along(ratio) {
        return {
          x: start.x + (end.x - start.x) * ratio,
          y: start.y + (end.y - start.y) * ratio
        };
      }

      var namePoint = along(0.55);
      var dx = namePoint.x - centreX;
      var dy = namePoint.y - centreY;
      var distanceFromCentre = Math.sqrt(dx * dx + dy * dy) || 1;
      builder.text(
        namePoint.x + (dx / distanceFromCentre) * 18,
        namePoint.y + (dy / distanceFromCentre) * 18 + 4,
        truncate(entry.name, 16),
        { fill: theme.muted, size: 11, weight: 650, anchor: 'middle' }
      );

      var low = along(0.14);
      builder.text(low.x, low.y + 12, formatNumber(extent[entry.axis].min), {
        fill: theme.muted, size: 9, anchor: 'middle'
      });
      builder.text(end.x, end.y + 12, formatNumber(extent[entry.axis].max), {
        fill: theme.muted, size: 9, anchor: 'middle'
      });
    });
    void origin;

    /* Categories keep the fixed hue order, so a colour means the same thing
     * here as in every other chart of the notebook. */
    var categories = [];
    points.forEach(function(point){
      if (point.series === undefined || point.series === null) return;
      var name = String(point.series);
      if (categories.indexOf(name) === -1) categories.push(name);
    });
    if (categories.length > MAX_SERIES) {
      categories = categories.slice(0, MAX_SERIES - 1).concat([settings.otherLabel || 'Other']);
    }

    var sizes = points.map(function(point){
      return typeof point.size === 'number' && isFinite(point.size) ? point.size : null;
    }).filter(function(value){ return value !== null; });
    var sizeMin = sizes.length ? Math.min.apply(null, sizes) : 0;
    var sizeMax = sizes.length ? Math.max.apply(null, sizes) : 0;

    var projected = points.map(function(point){
      var place = project(normalise(point.x, 'x'), normalise(point.y, 'y'), normalise(point.z, 'z'));
      var name = point.series === undefined || point.series === null ? null : String(point.series);
      var slot = name === null ? 0 : categories.indexOf(name);
      if (slot === -1) slot = categories.length - 1;
      var marker = 4;
      if (sizes.length && typeof point.size === 'number' && sizeMax > sizeMin) {
        marker = 3 + ((point.size - sizeMin) / (sizeMax - sizeMin)) * 6;
      }
      return {
        place: place,
        colour: palette[slot % palette.length],
        radius: marker * place.scale,
        series: name,
        point: point
      };
    }).sort(function(left, right){ return right.place.depth - left.place.depth; });

    projected.forEach(function(entry){
      var value = formatNumber(entry.point.x) + ' / ' + formatNumber(entry.point.y) +
        ' / ' + formatNumber(entry.point.z);
      builder.add(
        '<circle class="mark" cx="' + round(entry.place.x) + '" cy="' + round(entry.place.y) +
        '" r="' + round(Math.max(2, entry.radius)) + '" fill="' + entry.colour +
        '" fill-opacity="' + (0.55 + 0.45 * entry.place.scale / 1.4).toFixed(2) +
        '" stroke="' + theme.surface + '" stroke-width="1.5"' +
        ' data-label="' + escapeXml(entry.point.label || entry.series || '') + '"' +
        ' data-series="' + escapeXml((spec.axis && spec.axis.x) + ' / ' + (spec.axis && spec.axis.y) + ' / ' + (spec.axis && spec.axis.z)) + '"' +
        ' data-value="' + escapeXml(value) + '"/>'
      );
    });

    if (categories.length > 1) {
      var legendX = 16;
      var legendY = height - 10;
      categories.forEach(function(name, index){
        var label = truncate(name, 16);
        var itemWidth = 14 + textWidth(label, 11) + 14;
        if (legendX + itemWidth > width - 8 && index > 0) return;
        builder.add('<rect x="' + round(legendX) + '" y="' + round(legendY - 8) +
          '" width="9" height="9" rx="2" fill="' + palette[index % palette.length] + '"/>');
        builder.text(legendX + 14, legendY, label, { fill: theme.muted, size: 11 });
        legendX += itemWidth;
      });
    }

    return {
      svg: wrap(builder, width, height, spec.title || '3D scatter'),
      width: width,
      height: height,
      yaw: yaw,
      pitch: pitch,
      rotatable: true
    };
  }

  function wrap(builder, width, height, title) {
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ' + width + ' ' + height + '"' +
      ' width="100%" height="auto" preserveAspectRatio="xMidYMid meet" role="img"' +
      ' aria-label="' + escapeXml(title || 'chart') + '">' + builder.parts.join('') + '</svg>';
  }

  /* ------------------------------------------------------------- Facade */

  /* `spec` is the chart output emitted by a kernel:
   * {chart, title, labels, series:[{name, values}], stacked, horizontal, axis} */
  function render(spec, options) {
    var settings = options || {};
    var mode = settings.theme === 'light' ? 'light' : 'dark';
    var theme = THEMES[mode];
    /* An author-supplied colour replaces the validated order for the slots it
     * covers; the remaining slots keep their place in that order. */
    var palette = withOverrides(PALETTE[mode], spec.colors);
    var formatNumber = settings.formatNumber || defaultNumber;

    if (spec.chart === 'scatter3d') {
      if (!spec.points || !spec.points.length) return null;
    } else if (!spec.series || !spec.series.length) {
      return null;
    }

    if (spec.chart === 'pie') {
      return renderPie(spec, theme, palette, formatNumber, settings);
    }

    if (spec.chart === 'scatter3d') {
      return renderScatter3d(spec, theme, palette, formatNumber, settings);
    }

    var layout = measure(spec, theme, settings);
    var builder = new Builder();
    builder.add('<rect width="' + layout.width + '" height="' + layout.height + '" fill="' + theme.surface + '"/>');

    drawFrame(builder, spec, layout, theme, palette, formatNumber);

    if (spec.chart === 'bar') {
      renderBars(builder, spec, layout, theme, palette, formatNumber);
    } else if (spec.chart === 'line') {
      renderLines(builder, spec, layout, theme, palette, formatNumber, false);
    } else if (spec.chart === 'area') {
      renderLines(builder, spec, layout, theme, palette, formatNumber, true);
    } else if (spec.chart === 'scatter') {
      renderScatter(builder, spec, layout, theme, palette, formatNumber);
    } else {
      renderBars(builder, spec, layout, theme, palette, formatNumber);
    }

    drawLegend(builder, layout, theme, palette);

    if (spec.axis && spec.axis.x && !layout.horizontal) {
      builder.text(layout.width - 12, layout.height - (layout.series.length > 1 ? 30 : 6), spec.axis.x, {
        fill: theme.muted, size: 10, anchor: 'end'
      });
    }

    return { svg: wrap(builder, layout.width, layout.height, spec.title || spec.axis && spec.axis.y), width: layout.width, height: layout.height };
  }

  /* The table view behind every chart, which is also the relief the light
   * palette's contrast warning requires. */
  function toTable(spec) {
    if (spec.chart === 'scatter3d') {
      var axis = spec.axis || {};
      var columns = [axis.x || 'x', axis.y || 'y', axis.z || 'z'];
      var hasSeries = (spec.points || []).some(function(point){ return point.series != null; });
      if (hasSeries) columns.push(axis.series || 'series');
      return {
        columns: columns,
        rows: (spec.points || []).map(function(point){
          var row = [point.x, point.y, point.z];
          if (hasSeries) row.push(point.series);
          return row;
        })
      };
    }

    var labels = spec.labels || [];
    var columns = [(spec.axis && spec.axis.x) || 'label'].concat(spec.series.map(function(entry){ return entry.name; }));
    var rows = labels.map(function(label, index){
      return [label].concat(spec.series.map(function(entry){
        var value = entry.values[index];
        return typeof value === 'number' && !isNaN(value) ? value : null;
      }));
    });
    return { columns: columns, rows: rows };
  }

  return Object.freeze({
    render: render,
    renderScatter3d: renderScatter3d,
    toTable: toTable,
    palette: PALETTE,
    themes: THEMES,
    niceTicks: niceTicks,
    formatNumber: defaultNumber,
    maxSeries: MAX_SERIES
  });
});
