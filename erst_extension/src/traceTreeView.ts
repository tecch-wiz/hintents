import * as vscode from 'vscode';
import { Trace, TraceStep } from './erstClient';

export class TraceTreeDataProvider implements vscode.TreeDataProvider<TraceItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<TraceItem | undefined | null | void> = new vscode.EventEmitter<TraceItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<TraceItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private currentTrace: Trace | undefined;
    private searchQuery = '';

    constructor() { }

    refresh(trace: Trace): void {
        this.currentTrace = trace;
        this._onDidChangeTreeData.fire();
    }

    getCurrentTrace(): Trace | undefined {
        return this.currentTrace;
    }

    setSearchQuery(searchQuery: string): void {
        this.searchQuery = searchQuery;
        this._onDidChangeTreeData.fire();
    }

    getSearchQuery(): string {
        return this.searchQuery;
    }

    getTreeItem(element: TraceItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: TraceItem): Thenable<TraceItem[]> {
        if (!this.currentTrace) {
            return Promise.resolve([]);
        }

        if (element) {
            // Further details if expanded, but for now we just show steps
            return Promise.resolve([]);
        } else {
            return Promise.resolve(
                this.currentTrace.states.map(step => new TraceItem(step, this.searchQuery))
            );
        }
    }
}

export class TraceItem extends vscode.TreeItem {
    constructor(
        public readonly step: TraceStep,
        searchQuery: string
    ) {
        super(
            `${step.step}: ${step.operation}${step.function ? ` (${step.function})` : ''}`,
            vscode.TreeItemCollapsibleState.None
        );

        this.tooltip = `${this.label}`;
        this.description = step.error ? `Error: ${step.error}` : '';
        this.contextValue = 'traceStep';

        const matched = isStepMatch(step, searchQuery);

        if (step.error) {
            this.iconPath = new vscode.ThemeIcon('error', new vscode.ThemeColor('errorForeground'));
        } else if (matched) {
            this.iconPath = new vscode.ThemeIcon('search', new vscode.ThemeColor('charts.yellow'));
        } else {
            this.iconPath = new vscode.ThemeIcon('pass', new vscode.ThemeColor('debugIcon.startForeground'));
        }
    }
}

function isStepMatch(step: TraceStep, searchQuery: string): boolean {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
        return false;
    }

    const haystack = JSON.stringify(step).toLowerCase();
    return haystack.includes(query);
}
