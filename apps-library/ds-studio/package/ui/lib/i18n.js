/* Translation and locale-aware formatting.
 *
 * Every user-facing string lives in ui/locales/<lang>.json. Adding a language
 * is dropping one file next to the others and listing its tag in AVAILABLE:
 * no string is written into the markup or the logic.
 */
(function(root, factory){
  'use strict';
  var api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioI18n = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(){
  'use strict';

  var AVAILABLE = ['fr', 'en'];
  var FALLBACK = 'fr';

  function I18n() {
    this.locale = FALLBACK;
    this.catalog = {};
    this.fallbackCatalog = {};
    this.pluralRules = null;
    this.numberFormats = {};
    this.dateFormats = {};
  }

  I18n.prototype.negotiate = function(requested) {
    var tag = String(requested || '').toLowerCase();
    for (var i = 0; i < AVAILABLE.length; i += 1) {
      if (tag === AVAILABLE[i] || tag.indexOf(AVAILABLE[i] + '-') === 0) return AVAILABLE[i];
    }
    return FALLBACK;
  };

  I18n.prototype.use = function(locale, catalog, fallbackCatalog) {
    this.locale = locale;
    this.catalog = catalog || {};
    this.fallbackCatalog = fallbackCatalog || {};
    this.numberFormats = {};
    this.dateFormats = {};
    try {
      this.pluralRules = new Intl.PluralRules(locale);
    } catch (error) {
      this.pluralRules = null;
    }
  };

  I18n.prototype.raw = function(key) {
    if (Object.prototype.hasOwnProperty.call(this.catalog, key)) return this.catalog[key];
    if (Object.prototype.hasOwnProperty.call(this.fallbackCatalog, key)) return this.fallbackCatalog[key];
    return null;
  };

  /* A count triggers the plural lookup: "cell_one" / "cell_other", with the
   * category coming from Intl so languages with more forms just work. */
  I18n.prototype.t = function(key, params) {
    var values = params || {};
    var template = null;

    if (Object.prototype.hasOwnProperty.call(values, 'count') && this.pluralRules) {
      var category = this.pluralRules.select(Number(values.count) || 0);
      template = this.raw(key + '_' + category);
      if (template === null) template = this.raw(key + '_other');
    }
    if (template === null || template === undefined) template = this.raw(key);
    if (template === null || template === undefined) return key;

    var self = this;
    return String(template).replace(/\{(\w+)\}/g, function(match, name){
      if (!Object.prototype.hasOwnProperty.call(values, name)) return match;
      var value = values[name];
      return typeof value === 'number' ? self.number(value) : String(value);
    });
  };

  I18n.prototype.number = function(value, options) {
    if (value === null || value === undefined || (typeof value === 'number' && isNaN(value))) return '';
    var settings = options || {};
    var signature = JSON.stringify(settings);
    if (!this.numberFormats[signature]) {
      try {
        this.numberFormats[signature] = new Intl.NumberFormat(this.locale, settings);
      } catch (error) {
        this.numberFormats[signature] = { format: function(input){ return String(input); } };
      }
    }
    return this.numberFormats[signature].format(value);
  };

  /* Axis and cell values need to stay short without losing the magnitude. */
  I18n.prototype.compact = function(value) {
    if (typeof value !== 'number' || !isFinite(value)) return '';
    var abs = Math.abs(value);
    if (abs >= 1000) {
      return this.number(value, { notation: 'compact', maximumFractionDigits: 1 });
    }
    if (Number.isInteger(value)) return this.number(value);
    return this.number(value, { maximumFractionDigits: abs < 1 ? 4 : 2 });
  };

  I18n.prototype.date = function(value, options) {
    var date = value instanceof Date ? value : new Date(value);
    if (isNaN(date.getTime())) return '';
    var settings = options || { dateStyle: 'medium' };
    var signature = JSON.stringify(settings);
    if (!this.dateFormats[signature]) {
      try {
        this.dateFormats[signature] = new Intl.DateTimeFormat(this.locale, settings);
      } catch (error) {
        this.dateFormats[signature] = { format: function(input){ return input.toISOString(); } };
      }
    }
    return this.dateFormats[signature].format(date);
  };

  I18n.prototype.bytes = function(value) {
    var units = ['unitByte', 'unitKilobyte', 'unitMegabyte'];
    var size = Number(value) || 0;
    var index = 0;
    while (size >= 1024 && index < units.length - 1) {
      size /= 1024;
      index += 1;
    }
    return this.number(size, { maximumFractionDigits: index === 0 ? 0 : 1 }) + ' ' + this.t(units[index]);
  };

  I18n.prototype.duration = function(milliseconds) {
    var value = Number(milliseconds) || 0;
    if (value < 1000) return this.t('durationMs', { value: Math.round(value) });
    if (value < 60000) return this.t('durationS', { value: this.number(value / 1000, { maximumFractionDigits: 1 }) });
    return this.t('durationMin', { value: this.number(value / 60000, { maximumFractionDigits: 1 }) });
  };

  I18n.prototype.relative = function(timestamp) {
    var delta = Date.now() - Number(timestamp || 0);
    if (delta < 60000) return this.t('justNow');
    if (delta < 3600000) return this.t('minutesAgo', { count: Math.floor(delta / 60000) });
    if (delta < 86400000) return this.t('hoursAgo', { count: Math.floor(delta / 3600000) });
    return this.date(Number(timestamp), { dateStyle: 'short', timeStyle: 'short' });
  };

  /* Applies the catalogue to markup: data-i18n sets text content, and
   * data-i18n-<attribute> sets that attribute. */
  I18n.prototype.apply = function(root) {
    var self = this;
    var scope = root || document;

    scope.querySelectorAll('[data-i18n]').forEach(function(node){
      node.textContent = self.t(node.getAttribute('data-i18n'));
    });

    ['title', 'placeholder', 'aria-label', 'value'].forEach(function(attribute){
      scope.querySelectorAll('[data-i18n-' + attribute + ']').forEach(function(node){
        node.setAttribute(attribute, self.t(node.getAttribute('data-i18n-' + attribute)));
      });
    });
  };

  return Object.freeze({
    create: function(){ return new I18n(); },
    available: AVAILABLE.slice(),
    fallback: FALLBACK
  });
});
