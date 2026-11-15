import assert from 'node:assert/strict';
import { test, after } from 'node:test';
import { createNavigation, preserveHomeFocus } from './navigation.ts';

const original = { document: globalThis.document, input: globalThis.HTMLInputElement, select: globalThis.HTMLSelectElement };
class Input {}
class Select {}
globalThis.HTMLInputElement = Input;
globalThis.HTMLSelectElement = Select;

function fixture() {
  const buttons = [];
  const summaries = [];
  let modal = null;
  const document = { activeElement: null, querySelector: selector => selector === 'dialog[open]' ? modal : { querySelectorAll: query => query.includes('summary') ? [...buttons, ...summaries] : buttons } };
  globalThis.document = document;
  function control(x, y, disabled = false) {
    const value = {
      clicks: 0, scrolled: false, isConnected: true,
      getClientRects: () => disabled || !value.isConnected ? [] : [{}],
      getBoundingClientRect: () => ({ x, y, width: 80, height: 40 }),
      focus() { document.activeElement = value; },
      scrollIntoView() { value.scrolled = true; },
      click() { value.clicks++; },
    };
    return value;
  }
  function button(x, y, disabled = false) {
    const value = control(x, y, disabled);
    buttons.push(value);
    return value;
  }
  function summary(x, y) {
    const value = control(x, y);
    summaries.push(value);
    return value;
  }
  let backed = 0;
  let homed = 0;
  const navigation = createNavigation(() => backed++, () => homed++);
  return { document, button, summary, navigation, modal: values => modal = { querySelectorAll: () => values }, counts: () => ({ backed, homed }) };
}

test('directional navigation follows the row and scrolls the focused tile into view', () => {
  const f = fixture();
  const a = f.button(0, 0), b = f.button(100, 0), c = f.button(0, 100);
  a.focus();
  f.navigation.dispatch('right');
  assert.equal(f.document.activeElement, b);
  assert.equal(b.scrolled, true);
  f.navigation.dispatch('left');
  assert.equal(f.document.activeElement, a);
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, c);
  f.navigation.dispatch('activate');
  assert.equal(c.clicks, 1);
});

test('settings remote navigation enters the content, visits every control, and returns to categories', () => {
  const f = fixture();
  const display = f.button(0, 0), appearance = f.button(0, 100);
  const first = f.button(400, 300), second = f.button(700, 300), save = f.button(700, 500);
  const categories = { contains: el => [display, appearance].includes(el) };
  const content = { contains: el => [first, second, save].includes(el) };
  const page = { querySelector: () => display };
  for (const el of [display, appearance, first, second, save]) {
    el.closest = selector => selector === '.settings-page' ? page
      : selector === '.settings-categories' && categories.contains(el) ? categories
      : selector === '.settings-content' && content.contains(el) ? content : null;
  }
  display.focus();
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, appearance);
  display.focus();
  f.navigation.dispatch('right');
  assert.equal(f.document.activeElement, first);
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, second);
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, save);
  f.navigation.dispatch('activate');
  assert.equal(save.clicks, 1);
  f.navigation.dispatch('left');
  assert.equal(f.document.activeElement, display);
});

test('settings section tabs move down into their content instead of the next tab', () => {
  const f = fixture();
  const display = f.button(0, 0);
  const theme = f.button(400, 100), layout = f.button(520, 100);
  const first = f.button(400, 300), second = f.button(700, 300);
  const categories = { contains: el => el === display };
  const content = { contains: el => [theme, layout, first, second].includes(el) };
  const sections = { contains: el => [theme, layout].includes(el) };
  const page = { querySelector: () => display };
  for (const el of [display, theme, layout, first, second]) {
    el.closest = selector => selector === '.settings-page' ? page
      : selector === '.settings-categories' && categories.contains(el) ? categories
      : selector === '.settings-content' && content.contains(el) ? content
      : selector === '.settings-sections' && sections.contains(el) ? sections : null;
  }
  theme.focus();
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, first);
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, second);
});

test('modal navigation stays in the overlay and Tab wraps in both directions', () => {
  const f = fixture();
  const background = f.button(0, 0), a = f.button(0, 100), b = f.button(100, 100);
  f.modal([a, b]);
  background.focus();
  f.navigation.dispatch('right');
  assert.equal(f.document.activeElement, a);
  const event = { key: 'Tab', shiftKey: true, preventDefault() {} };
  f.navigation.keydown(event);
  assert.equal(f.document.activeElement, b);
  f.navigation.keydown({ ...event, shiftKey: false });
  assert.equal(f.document.activeElement, a);
});

test('text editing retains arrow and Backspace behavior while Escape still goes back', () => {
  const f = fixture();
  let prevented = false;
  const event = { target: new Input(), key: 'Backspace', preventDefault() { prevented = true; } };
  f.navigation.keydown(event);
  assert.equal(prevented, false);
  assert.equal(f.counts().backed, 0);
  f.navigation.keydown({ ...event, key: 'Escape' });
  assert.equal(f.counts().backed, 1);
  f.navigation.dispatch('home');
  assert.equal(f.counts().homed, 1);
});

test('home shortcut defaults to Ctrl+W and works while editing without repeats', () => {
  const f = fixture();
  let prevented = 0;
  const event = { target: new Input(), key: 'w', ctrlKey: true, preventDefault() { prevented++; } };
  f.navigation.keydown(event);
  f.navigation.keydown({ ...event, repeat: true });
  f.navigation.keydown({ ...event, altKey: true });
  assert.equal(f.counts().homed, 1);
  assert.equal(prevented, 2);
});

test('changing the home shortcut replaces the previous binding immediately', () => {
  fixture();
  let shortcut = 'Alt+Home';
  let homed = 0;
  const navigation = createNavigation(() => {}, () => homed++, () => shortcut);
  const event = { key: 'Home', altKey: true, preventDefault() {} };
  navigation.keydown(event);
  shortcut = 'Ctrl+J';
  navigation.keydown(event);
  navigation.keydown({ ...event, key: 'j', altKey: false, ctrlKey: true });
  navigation.keydown({ ...event, key: 'j', altKey: false, ctrlKey: true, defaultPrevented: true });
  assert.equal(homed, 2);
});

test('remote arrows adjust playback sliders and vertical navigation leaves them', () => {
  const f = fixture();
  const slider = f.button(0, 0), next = f.button(0, 100);
  Object.setPrototypeOf(slider, Input.prototype);
  Object.assign(slider, { type: 'range', min: '0', max: '100', step: '1', value: '99', disabled: false });
  const events = [];
  slider.dispatchEvent = event => events.push(event.type);
  slider.focus();
  f.navigation.dispatch('right');
  assert.equal(slider.value, '100');
  f.navigation.dispatch('right');
  assert.equal(slider.value, '100');
  f.navigation.dispatch('left');
  assert.equal(slider.value, '99');
  assert.deepEqual(events, ['input', 'change', 'input', 'change', 'input', 'change']);
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, next);
});

test('remote activate submits the form for a focused search field', () => {
  const f = fixture();
  const field = f.button(0, 0);
  let submitted = 0;
  Object.setPrototypeOf(field, Input.prototype);
  Object.assign(field, { type: 'search', form: { requestSubmit: () => submitted++ } });
  field.focus();
  f.navigation.dispatch('activate');
  assert.equal(submitted, 1);
  assert.equal(field.clicks, 0);
});

test('remote changes dropdown choices, skips disabled outputs, and leaves with vertical navigation', () => {
  const f = fixture();
  const select = f.button(0, 0), next = f.button(0, 100);
  Object.setPrototypeOf(select, Select.prototype);
  Object.assign(select, { selectedIndex: 0, options: [{}, { disabled: true }, {}], disabled: false });
  const events = [];
  select.dispatchEvent = event => events.push(event.type);
  select.focus();
  f.navigation.dispatch('right');
  assert.equal(select.selectedIndex, 2);
  f.navigation.dispatch('right');
  assert.equal(select.selectedIndex, 2);
  f.navigation.dispatch('left');
  assert.equal(select.selectedIndex, 0);
  assert.deepEqual(events, ['input', 'change', 'input', 'change']);
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, next);
});

test('details summary controls join the remote focus order', () => {
  const f = fixture();
  const first = f.button(0, 0), disclosure = f.summary(0, 100);
  first.focus();
  f.navigation.dispatch('down');
  assert.equal(f.document.activeElement, disclosure);
  f.navigation.dispatch('activate');
  assert.equal(disclosure.clicks, 1);
});

test('closing settings restores visible focus when its original section was hidden', () => {
  const f = fixture();
  const hidden = f.button(0, 0), visible = f.button(100, 0);
  hidden.isConnected = false;
  f.navigation.focus(hidden);
  assert.equal(f.document.activeElement, visible);
  f.navigation.dispatch('right');
  assert.equal(f.document.activeElement, visible);
});

test('Music entry prioritizes usable playback controls over the header and skips hidden controls', () => {
  const f = fixture();
  const header = f.button(0, 0), hidden = f.button(0, 100, true), play = f.button(100, 100);
  header.matches = selector => selector === '.music-home button';
  hidden.matches = play.matches = selector => selector === '.music-home .music-now-actions button';
  header.focus();
  f.navigation.focusMusic();
  assert.equal(f.document.activeElement, play);
  f.navigation.dispatch('activate');
  assert.equal(play.clicks, 1);
});

test('Music entry focuses setup when playback is unavailable and can be repeated', () => {
  const f = fixture();
  const header = f.button(0, 0), start = f.button(0, 100);
  header.matches = selector => selector === '.music-home button';
  start.matches = selector => selector === '.music-home .music-starting button';
  f.navigation.focusMusic();
  assert.equal(f.document.activeElement, start);
  start.isConnected = false;
  f.navigation.focusMusic();
  assert.equal(f.document.activeElement, header);
});

after(() => {
  if (original.document === undefined) delete globalThis.document; else globalThis.document = original.document;
  if (original.input === undefined) delete globalThis.HTMLInputElement; else globalThis.HTMLInputElement = original.input;
  if (original.select === undefined) delete globalThis.HTMLSelectElement; else globalThis.HTMLSelectElement = original.select;
});

function homeFocusFixture() {
  const previousDocument = globalThis.document;
  const previousObserver = globalThis.MutationObserver;
  const listeners = new Map();
  let notify;
  let disconnected = false;
  const rows = [];
  const body = {};
  const doc = {
    body, activeElement: body,
    addEventListener: (name, callback) => listeners.set(name, callback),
    removeEventListener: name => listeners.delete(name),
    querySelector: selector => selector === '#home-button' ? home : null,
  };
  const home = { focus() { doc.activeElement = home; listeners.get('focusin')?.({ target: home }); }, scrollIntoView() {}, closest: () => null };
  const root = { isConnected: true, contains: row => rows.includes(row), querySelectorAll: () => rows };
  globalThis.document = doc;
  globalThis.MutationObserver = class {
    constructor(callback) { notify = callback; }
    observe() {}
    disconnect() { disconnected = true; }
  };
  function row(id) {
    const value = { dataset: { homeInstance: id }, controls: [], querySelectorAll() { return this.controls.filter(control => control.isConnected && !control.disabled); } };
    rows.push(value);
    return value;
  }
  function button(owner, key) {
    const value = {
      isConnected: true, disabled: false, dataset: { focusKey: key },
      closest: selector => selector === '[data-home-instance]' ? owner : null,
      matches: () => value.disabled,
      getClientRects: () => value.isConnected ? [{}] : [],
      focus() { doc.activeElement = value; listeners.get('focusin')?.({ target: value }); },
      scrollIntoView() {},
    };
    owner.controls.push(value);
    return value;
  }
  const dispose = preserveHomeFocus(root);
  return { doc, rows, row, button, home, notify: () => notify(), dispose,
    disconnected: () => disconnected,
    cleanup() { dispose(); globalThis.document = previousDocument; globalThis.MutationObserver = previousObserver; },
  };
}

test('Home replacement restores the same instance/item and preserves focus across moves', () => {
  const f = homeFocusFixture();
  try {
    const apps = f.row('personal-apps');
    const old = f.button(apps, 'Spotify');
    old.focus();
    old.isConnected = false;
    f.doc.activeElement = f.doc.body;
    f.button(apps, 'YouTube');
    const replacement = f.button(apps, 'Spotify');
    f.notify();
    assert.equal(f.doc.activeElement, replacement);
    f.doc.activeElement = f.doc.body;
    f.notify();
    assert.equal(f.doc.activeElement, replacement);
    f.home.focus();
    replacement.isConnected = false;
    f.notify();
    assert.equal(f.doc.activeElement, f.home);
    f.dispose();
    assert.equal(f.disconnected(), true);
  } finally { f.cleanup(); }
});

test('Home removal chooses a nearby usable row and falls back to core navigation', () => {
  const f = homeFocusFixture();
  try {
    const first = f.row('first');
    const middle = f.row('middle');
    const last = f.row('last');
    f.button(first, 'one');
    const removed = f.button(middle, 'two');
    const next = f.button(last, 'three');
    removed.focus();
    removed.isConnected = false;
    f.rows.splice(1, 1);
    f.doc.activeElement = f.doc.body;
    f.notify();
    assert.equal(f.doc.activeElement, next);
    next.disabled = true;
    first.controls[0].isConnected = false;
    f.doc.activeElement = f.doc.body;
    f.notify();
    assert.equal(f.doc.activeElement, f.home);
  } finally { f.cleanup(); }
});
