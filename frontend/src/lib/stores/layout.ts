const STORAGE_KEY = 'gitv-layout';

interface LayoutState {
	sidebarWidth: number;
	detailPanelHeight: number;
	rightPanelWidth: number;
	graphWidth: number;
	sidebarCollapsed: boolean;
	windowWidth: number;
	windowHeight: number;
	windowX: number;
	windowY: number;
}

const DEFAULTS: LayoutState = {
	sidebarWidth: 220,
	detailPanelHeight: 500,
	rightPanelWidth: 240,
	graphWidth: 200,
	sidebarCollapsed: false,
	windowWidth: 1280,
	windowHeight: 800,
	windowX: -1,
	windowY: -1
};

const MIN: Partial<LayoutState> = {
	sidebarWidth: 150,
	detailPanelHeight: 200,
	rightPanelWidth: 160,
	graphWidth: 100
};

const MAX_FRACTIONS: Partial<LayoutState> = {
	sidebarWidth: 0.4,
	detailPanelHeight: 0.5,
	rightPanelWidth: 0.4
};

function clamp(value: number, min: number, max: number): number {
	return Math.max(min, Math.min(max, value));
}

function loadLayout(): LayoutState {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return { ...DEFAULTS };
		const saved = JSON.parse(raw) as Partial<LayoutState>;
		return { ...DEFAULTS, ...saved };
	} catch {
		return { ...DEFAULTS };
	}
}

function saveLayout(state: LayoutState) {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
	} catch {
		// storage full or unavailable — non-critical
	}
}

let currentLayout = loadLayout();

export function getLayout(): LayoutState {
	return { ...currentLayout };
}

export function getClampedLayout(): LayoutState {
	const vw = typeof window !== 'undefined' ? window.innerWidth : 1280;
	const vh = typeof window !== 'undefined' ? window.innerHeight : 800;
	const layout = getLayout();

	return {
		...layout,
		sidebarWidth: clamp(
			layout.sidebarWidth,
			MIN.sidebarWidth ?? 150,
			Math.floor(vw * (MAX_FRACTIONS.sidebarWidth ?? 0.4))
		),
		detailPanelHeight: clamp(
			layout.detailPanelHeight,
			MIN.detailPanelHeight ?? 200,
			Math.floor(vh * 0.9)
		),
		rightPanelWidth: clamp(
			layout.rightPanelWidth,
			MIN.rightPanelWidth ?? 160,
			Math.floor(vw * (MAX_FRACTIONS.rightPanelWidth ?? 0.4))
		),
		graphWidth: clamp(layout.graphWidth, MIN.graphWidth ?? 100, Math.floor(vw * 0.4))
	};
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

function debouncedSave() {
	if (saveTimer) clearTimeout(saveTimer);
	saveTimer = setTimeout(() => {
		saveLayout(currentLayout);
		saveTimer = null;
	}, 300);
}

export function updateLayout(partial: Partial<LayoutState>) {
	currentLayout = { ...currentLayout, ...partial };
	debouncedSave();
}

export async function restoreWindowGeometry() {
	try {
		const { getCurrentWindow } = await import('@tauri-apps/api/window');
		const win = getCurrentWindow();
		const layout = getLayout();

		if (layout.windowWidth > 0 && layout.windowHeight > 0) {
			const { LogicalSize } = await import('@tauri-apps/api/window');
			await win.setSize(new LogicalSize(layout.windowWidth, layout.windowHeight));
		}

		if (layout.windowX >= 0 && layout.windowY >= 0) {
			const { LogicalPosition } = await import('@tauri-apps/api/window');
			await win.setPosition(new LogicalPosition(layout.windowX, layout.windowY));
		}
	} catch {
		// not in Tauri context or API unavailable
	}
}

export async function saveWindowGeometry() {
	try {
		const { getCurrentWindow } = await import('@tauri-apps/api/window');
		const win = getCurrentWindow();
		const pos = await win.outerPosition();
		const size = await win.outerSize();
		updateLayout({
			windowX: pos.x,
			windowY: pos.y,
			windowWidth: size.width,
			windowHeight: size.height
		});
	} catch {
		// not in Tauri context
	}
}
