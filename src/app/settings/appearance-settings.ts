import { Component, input, output } from '@angular/core';
import { ColorTheme, colorThemes, ThemeChoice } from '../core/color-theme';

@Component({
  selector: 'app-appearance-settings',
  template: `
    <section aria-labelledby="appearance-title" [attr.aria-busy]="busy()">
      <h3 id="appearance-title">Appearance</h3>
      <p id="appearance-help">Pick your palette. Make this little space yours.</p>
      <div class="mode-switch" role="group" aria-label="Color mode">
        <button type="button" [attr.aria-pressed]="!light()" [disabled]="busy()"
          (click)="choose.emit({ light: false, palette: palette() })">Dark</button>
        <button type="button" [attr.aria-pressed]="light()" [disabled]="busy()"
          (click)="choose.emit({ light: true, palette: palette() })">Light</button>
      </div>
      <div class="palettes" role="group" aria-label="Color palette" aria-describedby="appearance-help">
        @for (theme of themes; track theme.id) {
          <button type="button" class="palette" [attr.aria-pressed]="palette() === theme.id"
            [attr.aria-label]="theme.name + ' palette'" [disabled]="busy()"
            (click)="choose.emit({ light: light(), palette: theme.id })">
            <span class="theme-preview" [class.light]="light()" [attr.data-palette]="theme.id" aria-hidden="true">
              <span class="preview-hero"></span>
              <span class="preview-row"><i></i><b></b></span>
              <span class="preview-row"><i></i><b></b></span>
            </span>
            <span class="palette-name">{{ theme.name }}<span aria-hidden="true">{{ palette() === theme.id ? '✓' : '' }}</span></span>
            <span class="palette-description">{{ theme.description }}</span>
          </button>
        }
      </div>
      <p class="save-state" role="status">{{ busy() ? 'Saving appearance…' : message() }}</p>
      @if (error()) { <p class="error" role="alert">{{ error() }}</p> }
    </section>
  `,
  styleUrl: './appearance-settings.css',
})
export class AppearanceSettings {
  readonly palette = input<ColorTheme>('lime');
  readonly light = input(false);
  readonly busy = input(false);
  readonly error = input('');
  readonly message = input('');
  readonly choose = output<ThemeChoice>();
  protected readonly themes = colorThemes;
}
