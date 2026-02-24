import * as vscode from 'vscode';
import { Trace, TraceStep } from './erstClient';

export class TraceTreeDataProvider implements vscode.TreeDataProvider<TraceItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<TraceItem | undefined | null | void> = new vscode.EventEmitter<TraceItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<TraceItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private currentTrace: Trace | undefined;

    constructor() { }

    refresh(trace: Trace): void {
        this.currentTrace = trace;
        this._onDidChangeTreeData.fire();
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
                this.currentTrace.states.map(step => new TraceItem(step))
            );
        }
    }
}

export class TraceItem extends vscode.TreeItem {
    constructor(
        public readonly step: TraceStep
    ) {
        super(
            `${step.step}: ${step.operation}${step.function ? ` (${step.function})` : ''}`,
            vscode.TreeItemCollapsibleState.None
        );

        // Build budget metrics display
        const budgetParts: string[] = [];
        if (step.cpu_delta !== undefined && step.cpu_delta > 0) {
            budgetParts.push(`CPU: ${this.formatNumber(step.cpu_delta)}`);
        }
        if (step.memory_delta !== undefined && step.memory_delta > 0) {
            budgetParts.push(`Mem: ${this.formatBytes(step.memory_delta)}`);
        }
        const budgetInfo = budgetParts.length > 0 ? ` [${budgetParts.join(', ')}]` : '';

        this.tooltip = `${this.label}${budgetInfo}`;
        this.description = step.error ? `Error: ${step.error}` : budgetInfo;
        this.contextValue = 'traceStep';

        if (step.error) {
            this.iconPath = new vscode.ThemeIcon('error', new vscode.ThemeColor('errorForeground'));
        } else {
            this.iconPath = new vscode.ThemeIcon('pass', new vscode.ThemeColor('debugIcon.startForeground'));
        }
    }

    private formatNumber(num: number): string {
        if (num >= 1000000) {
            return `${(num / 1000000).toFixed(2)}M`;
        } else if (num >= 1000) {
            return `${(num / 1000).toFixed(2)}K`;
        }
        return num.toString();
    }

    private formatBytes(bytes: number): string {
        if (bytes >= 1048576) {
            return `${(bytes / 1048576).toFixed(2)}MB`;
        } else if (bytes >= 1024) {
            return `${(bytes / 1024).toFixed(2)}KB`;
        }
        return `${bytes}B`;
    }
}
