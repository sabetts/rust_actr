export class AgiWindow {
  constructor(root) {
    this.root = root;
    this.title = root.querySelector(".agi-title");
    this.surface = root.querySelector(".agi-surface");
    this.items = new Map();
    this.markers = new Map();
    this.reset();
  }

  reset() {
    this.items.clear();
    this.markers.clear();
    this.surface.replaceChildren();
    this.surface.style.width = "300px";
    this.surface.style.height = "300px";
    this.title.textContent = "";
    this.root.hidden = true;
  }

  handle(args) {
    if (!Array.isArray(args) || !args.length) return;
    const [op] = args;
    switch (op) {
      case "open":
        this.open(args[1], args[2], args[3], args[4], args[5]);
        break;
      case "select":
        this.title.textContent = String(args[1] ?? "");
        break;
      case "close":
        this.reset();
        break;
      case "remove":
        this.remove(args[2]);
        break;
      case "text":
        this.text(args[2], args[3], args[4], args[5], args[6], args[7]);
        break;
      case "button":
        this.button(args[2], args[3], args[4], args[5], args[6], args[7], args[8]);
        break;
      case "line":
        this.line(args[2], args[3], args[4], args[5], args[6], args[7]);
        break;
      case "image":
        this.image(args[2], args[3], args[4], args[5], args[6], args[7], args[8]);
        break;
      case "attention":
        this.marker("attention", args[2], args[3], args[4], args[5]);
        break;
      case "clearattention":
        this.clearMarker("attention");
        break;
      case "cursor":
        this.marker("cursor", args[2], args[3], args[4], args[5]);
        break;
      case "clearcursor":
        this.clearMarker("cursor");
        break;
      default:
        break;
    }
  }

  open(title, x, y, width, height) {
    this.reset();
    this.title.textContent = String(title ?? "");
    this.surface.style.width = `${Number(width) || 300}px`;
    this.surface.style.height = `${Number(height) || 300}px`;
    this.root.dataset.x = String(x ?? 0);
    this.root.dataset.y = String(y ?? 0);
    this.root.hidden = false;
  }

  addItem(name, element) {
    const key = String(name ?? "");
    if (key) this.items.set(key, element);
    this.surface.append(element);
  }

  text(name, x, y, text, color, size) {
    const element = document.createElement("span");
    element.className = "agi-text";
    element.textContent = String(text ?? "");
    element.style.left = `${Number(x) || 0}px`;
    element.style.top = `${Number(y) || 0}px`;
    element.style.color = String(color || "black");
    element.style.fontSize = `${Number(size) || 12}px`;
    this.addItem(name, element);
  }

  button(name, x, y, width, height, text, color) {
    const element = document.createElement("button");
    element.className = "agi-button";
    element.type = "button";
    element.textContent = String(text ?? "");
    element.style.left = `${Number(x) || 0}px`;
    element.style.top = `${Number(y) || 0}px`;
    element.style.width = `${Number(width) || 75}px`;
    element.style.height = `${Number(height) || 20}px`;
    element.style.background = String(color || "gray");
    this.addItem(name, element);
  }

  line(name, x1, y1, x2, y2, color) {
    const left = Math.min(Number(x1) || 0, Number(x2) || 0);
    const top = Math.min(Number(y1) || 0, Number(y2) || 0);
    const dx = (Number(x2) || 0) - (Number(x1) || 0);
    const dy = (Number(y2) || 0) - (Number(y1) || 0);
    const length = Math.hypot(dx, dy);
    const angle = Math.atan2(dy, dx);
    const element = document.createElement("div");
    element.className = "agi-line";
    element.style.left = `${left}px`;
    element.style.top = `${top}px`;
    element.style.width = `${length}px`;
    element.style.background = String(color || "black");
    element.style.transform = `rotate(${angle}rad)`;
    this.addItem(name, element);
  }

  image(name, x, y, file, width, height) {
    const element = document.createElement("img");
    element.className = "agi-image";
    element.alt = "";
    element.src = `./AGI-images/${String(file ?? "")}`;
    element.style.left = `${Number(x) || 0}px`;
    element.style.top = `${Number(y) || 0}px`;
    element.style.width = `${Number(width) || 50}px`;
    element.style.height = `${Number(height) || 50}px`;
    this.addItem(name, element);
  }

  marker(kind, model, x, y, color) {
    this.clearMarker(kind);
    const element = document.createElement("div");
    element.className = `agi-${kind}`;
    element.style.left = `${Number(x) || 0}px`;
    element.style.top = `${Number(y) || 0}px`;
    element.style.background = String(color || "red");
    element.dataset.model = String(model ?? "");
    this.surface.append(element);
    this.markers.set(kind, element);
  }

  clearMarker(kind) {
    const marker = this.markers.get(kind);
    if (marker) marker.remove();
    this.markers.delete(kind);
  }

  remove(name) {
    const key = String(name ?? "");
    const element = this.items.get(key);
    if (element) element.remove();
    this.items.delete(key);
  }
}
