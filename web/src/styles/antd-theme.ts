/**
 * Ant Design Vue Theme Configuration
 *
 * Maps business design tokens to Ant Design component tokens.
 * This ensures antd components (buttons, modals, inputs) use the same
 * visual language as custom business components.
 */

export const antdTheme = {
  token: {
    // Primary color - maps to business primary color
    colorPrimary: '#26a69a',

    // Border radius - maps to business radius tokens
    borderRadius: 4,
    borderRadiusLG: 6,
    borderRadiusSM: 3,

    // Font
    fontSize: 13,
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',

    // Dark theme colors
    colorBgContainer: '#1e1e1e',
    colorBgElevated: '#252525',
    colorBorder: 'rgba(255, 255, 255, 0.12)',
    colorText: '#e0e0e0',
    colorTextSecondary: '#b0b0b0',
    colorTextTertiary: '#757575',

    // Success/Warning/Error - maps to business status colors
    colorSuccess: '#66bb6a',
    colorWarning: '#ffa726',
    colorError: '#ef5350',
    colorInfo: '#42a5f5',
  },

  components: {
    Button: {
      controlHeight: 28,
      paddingContentHorizontal: 12,
    },
    Input: {
      controlHeight: 28,
      paddingBlock: 4,
    },
    Modal: {
      contentBg: '#252525',
      headerBg: '#252525',
    },
  },
}
