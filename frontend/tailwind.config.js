/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.rs"],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'Helvetica Neue', 'Arial', 'Noto Sans', 'sans-serif'],
        display: ['Outfit', 'sans-serif'],
      },
      colors: {
        surface: {
          DEFAULT: 'rgb(var(--tk-surface-base) / <alpha-value>)',
          base: 'rgb(var(--tk-surface-base) / <alpha-value>)',
          elevated: 'rgb(var(--tk-surface-elevated) / <alpha-value>)',
          muted: 'rgb(var(--tk-surface-muted) / <alpha-value>)',
          subtle: 'rgb(var(--tk-surface-muted) / <alpha-value>)',
        },
        text: {
          primary: 'rgb(var(--tk-text-primary) / <alpha-value>)',
          secondary: 'rgb(var(--tk-text-secondary) / <alpha-value>)',
          muted: 'rgb(var(--tk-text-muted) / <alpha-value>)',
          inverse: 'rgb(var(--tk-text-inverse) / <alpha-value>)',
        },
        fg: {
          DEFAULT: 'rgb(var(--tk-text-primary) / <alpha-value>)',
          muted: 'rgb(var(--tk-text-muted) / <alpha-value>)',
        },
        border: {
          DEFAULT: 'rgb(var(--tk-border-subtle) / <alpha-value>)',
          subtle: 'rgb(var(--tk-border-subtle) / <alpha-value>)',
          strong: 'rgb(var(--tk-border-strong) / <alpha-value>)',
        },
        form: {
          control: {
            bg: 'rgb(var(--tk-form-control-bg) / <alpha-value>)',
            text: 'rgb(var(--tk-form-control-text) / <alpha-value>)',
            border: 'rgb(var(--tk-form-control-border) / <alpha-value>)',
            placeholder: 'rgb(var(--tk-form-control-placeholder) / <alpha-value>)',
          },
        },
        action: {
          primary: {
            bg: 'rgb(var(--tk-action-primary-bg) / <alpha-value>)',
            'bg-hover': 'rgb(var(--tk-action-primary-bg-hover) / <alpha-value>)',
            text: 'rgb(var(--tk-action-primary-text) / <alpha-value>)',
            border: 'rgb(var(--tk-action-primary-border) / <alpha-value>)',
            'border-hover': 'rgb(var(--tk-action-primary-border-hover) / <alpha-value>)',
            focus: 'rgb(var(--tk-action-primary-focus) / <alpha-value>)',
          },
          secondary: {
            bg: 'rgb(var(--tk-action-secondary-bg) / <alpha-value>)',
            'bg-hover': 'rgb(var(--tk-action-secondary-bg-hover) / <alpha-value>)',
            text: 'rgb(var(--tk-action-secondary-text) / <alpha-value>)',
            border: 'rgb(var(--tk-action-secondary-border) / <alpha-value>)',
            'border-hover': 'rgb(var(--tk-action-secondary-border-hover) / <alpha-value>)',
            focus: 'rgb(var(--tk-action-secondary-focus) / <alpha-value>)',
          },
          danger: {
            bg: 'rgb(var(--tk-action-danger-bg) / <alpha-value>)',
            'bg-hover': 'rgb(var(--tk-action-danger-bg-hover) / <alpha-value>)',
            text: 'rgb(var(--tk-action-danger-text) / <alpha-value>)',
            border: 'rgb(var(--tk-action-danger-border) / <alpha-value>)',
            'border-hover': 'rgb(var(--tk-action-danger-border-hover) / <alpha-value>)',
            focus: 'rgb(var(--tk-action-danger-focus) / <alpha-value>)',
          },
          ghost: {
            'bg-hover': 'rgb(var(--tk-action-ghost-bg-hover) / <alpha-value>)',
            text: 'rgb(var(--tk-action-ghost-text) / <alpha-value>)',
          },
        },
        state: {
          disabled: {
            bg: 'rgb(var(--tk-state-disabled-bg) / <alpha-value>)',
            text: 'rgb(var(--tk-state-disabled-text) / <alpha-value>)',
            border: 'rgb(var(--tk-state-disabled-border) / <alpha-value>)',
          },
        },
        status: {
          success: {
            bg: 'rgb(var(--tk-status-success-bg))',
            border: 'rgb(var(--tk-status-success-border) / <alpha-value>)',
            text: 'rgb(var(--tk-status-success-text) / <alpha-value>)',
          },
          error: {
            bg: 'rgb(var(--tk-status-error-bg))',
            border: 'rgb(var(--tk-status-error-border) / <alpha-value>)',
            text: 'rgb(var(--tk-status-error-text) / <alpha-value>)',
          },
          warning: {
            bg: 'rgb(var(--tk-status-warning-bg))',
            border: 'rgb(var(--tk-status-warning-border))',
            text: 'rgb(var(--tk-status-warning-text) / <alpha-value>)',
          },
          info: {
            bg: 'rgb(var(--tk-status-info-bg))',
            border: 'rgb(var(--tk-status-info-border) / <alpha-value>)',
            text: 'rgb(var(--tk-status-info-text) / <alpha-value>)',
          },
          neutral: {
            bg: 'rgb(var(--tk-status-neutral-bg) / <alpha-value>)',
            border: 'rgb(var(--tk-status-neutral-border) / <alpha-value>)',
            text: 'rgb(var(--tk-status-neutral-text) / <alpha-value>)',
          },
          attendance: {
            clock_in: 'rgb(var(--tk-status-attendance-clock-in) / <alpha-value>)',
            break: 'rgb(var(--tk-status-attendance-break) / <alpha-value>)',
            clock_out: 'rgb(var(--tk-status-attendance-clock-out) / <alpha-value>)',
            not_started: 'rgb(var(--tk-status-attendance-not-started) / <alpha-value>)',
            'text-active': 'rgb(var(--tk-status-attendance-text-active) / <alpha-value>)',
            'text-inactive': 'rgb(var(--tk-status-attendance-text-inactive) / <alpha-value>)',
          },
        },
        overlay: {
          backdrop: 'rgb(var(--tk-overlay-backdrop))',
        },
        link: {
          DEFAULT: 'rgb(var(--tk-link-default) / <alpha-value>)',
          hover: 'rgb(var(--tk-link-hover) / <alpha-value>)',
        },
        primary: {
          DEFAULT: 'rgb(var(--tk-action-primary-bg) / <alpha-value>)',
          strong: 'rgb(var(--tk-action-primary-bg-hover) / <alpha-value>)',
          subtle: 'rgb(var(--tk-primary-subtle) / <alpha-value>)',
        },
        neutral: {
          DEFAULT: 'rgb(var(--tk-action-secondary-bg) / <alpha-value>)',
          strong: 'rgb(var(--tk-action-secondary-bg-hover) / <alpha-value>)',
        },
        danger: {
          DEFAULT: 'rgb(var(--tk-action-danger-bg) / <alpha-value>)',
          strong: 'rgb(var(--tk-action-danger-bg-hover) / <alpha-value>)',
          subtle: 'rgb(var(--tk-status-error-bg))',
        },
        success: {
          DEFAULT: 'rgb(var(--tk-status-success-text) / <alpha-value>)',
          strong: 'rgb(var(--tk-status-success-text) / <alpha-value>)',
          subtle: 'rgb(var(--tk-status-success-bg))',
        },
        warning: {
          DEFAULT: 'rgb(var(--tk-status-warning-text) / <alpha-value>)',
          strong: 'rgb(var(--tk-status-warning-text) / <alpha-value>)',
          subtle: 'rgb(var(--tk-status-warning-bg))',
        },
        brand: {
          50: '#f0f4ff',
          100: '#e0e9fe',
          200: '#c1d3fe',
          300: '#92b2fd',
          400: '#5c89f8',
          500: '#375df1',
          600: '#253ee6',
          700: '#1e30d3',
          800: '#1e28ab',
          900: '#1e2888',
          950: '#121753',
        },
        slate: {
          950: '#020617',
        }
      },
      boxShadow: {
        'premium': '0 4px 20px -2px rgba(0, 0, 0, 0.05), 0 2px 10px -2px rgba(0, 0, 0, 0.03)',
        'premium-hover': '0 10px 30px -5px rgba(0, 0, 0, 0.08), 0 4px 15px -5px rgba(0, 0, 0, 0.05)',
      },
      animation: {
        'fade-in': 'fadeIn 0.5s ease-out',
        'pop-in': 'popIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1)',
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        popIn: {
          '0%': { opacity: '0', transform: 'scale(0.95)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        }
      }
    },
  },
  plugins: [],
};
