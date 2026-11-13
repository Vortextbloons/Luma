import assert from 'node:assert/strict';
import { test, after } from 'node:test';
import { createNavigation } from './navigation.ts';

const original = { document: globalThis.document, input: globalThis.HTMLInputElement, select: globalThis.HTMLSelectElement };
class Input {}
class Select {}
globalThis.HTMLInputElement = Input;
globalThis.HTMLSelectElement = Select;

function fixture() {
  const buttons = [];
  let modal = null;
  const document = { activeElement: null, querySelector: selector => selector === 'dialog[open]' ? modal : { querySelectorAll: () => buttons } };
  globalThis.document = document;
  function button(x, y, disabled = false) {
    const value = {
      clicks: 0, scrolled: false, isConnected: true,
      getClientRects: () => disabled || !value.isConnected ? [] : [{}],
      getBoundingClientRect: () => ({ x, y, width: 80, height: 40 }),
      focus() { document.activeElement = value; },
      scrollIntoView() { value.scrolled = true; },
      click() { value.clicks++; },
    };
    buttons.push(value);
    return value;
  }
  let backed = 0;
  let homed = 0;
  const navigation = createNavigation(() => backed++, () => homed++);
  return { document, button, navigation, modal: values => modal = { querySelectorAll: () => values }, counts: () => ({ backed, homed }) };
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

test('closing settings restores visible focus when its original section was hidden', () => {
  const f = fixture();
  const hidden = f.button(0, 0), visible = f.button(100, 0);
  hidden.isConnected = false;
  f.navigation.focus(hidden);
  assert.equal(f.document.activeElement, visible);
  f.navigation.dispatch('right');
  assert.equal(f.document.activeElement, visible);
});

after(() => {
  if (original.document === undefined) delete globalThis.document; else globalThis.document = original.document;
  if (original.input === undefined) delete globalThis.HTMLInputElement; else globalThis.HTMLInputElement = original.input;
  if (original.select === undefined) delete globalThis.HTMLSelectElement; else globalThis.HTMLSelectElement = original.select;
});
