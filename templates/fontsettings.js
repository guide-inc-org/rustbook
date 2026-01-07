// Font Settings Plugin - Font size and theme toggle functionality
// Based on HonKit fontsettings plugin

(function() {
    'use strict';

    // Storage keys
    var STORAGE_KEY_FONT_SIZE = 'guidebook-font-size';
    var STORAGE_KEY_THEME = 'guidebook-theme';

    // Font size configuration
    var FONT_SIZES = [12, 14, 16, 18, 20, 22, 24];
    var DEFAULT_FONT_SIZE_INDEX = 2; // 16px

    // Theme configuration
    var THEMES = {
        white: {
            background: '#ffffff',
            color: '#333333',
            sidebarBg: '#fafafa',
            sidebarBorder: '#e8e8e8',
            codeBg: '#f6f8fa',
            linkColor: '#4183c4'
        },
        sepia: {
            background: '#f4ecd8',
            color: '#5f4b32',
            sidebarBg: '#ede6d4',
            sidebarBorder: '#d4c9b0',
            codeBg: '#ebe4d0',
            linkColor: '#704214'
        },
        night: {
            background: '#1c1c1c',
            color: '#c8c8c8',
            sidebarBg: '#262626',
            sidebarBorder: '#3a3a3a',
            codeBg: '#2d2d2d',
            linkColor: '#6cb2eb'
        }
    };

    var DEFAULT_THEME = 'white';

    // Get current font size index
    function getFontSizeIndex() {
        var stored = localStorage.getItem(STORAGE_KEY_FONT_SIZE);
        if (stored !== null) {
            var index = parseInt(stored, 10);
            if (!isNaN(index) && index >= 0 && index < FONT_SIZES.length) {
                return index;
            }
        }
        return DEFAULT_FONT_SIZE_INDEX;
    }

    // Get current theme
    function getTheme() {
        var stored = localStorage.getItem(STORAGE_KEY_THEME);
        if (stored && THEMES[stored]) {
            return stored;
        }
        return DEFAULT_THEME;
    }

    // Apply font size
    function applyFontSize(index) {
        var size = FONT_SIZES[index];
        document.documentElement.style.setProperty('--book-font-size', size + 'px');
        document.querySelector('.markdown-section').style.fontSize = size + 'px';
        localStorage.setItem(STORAGE_KEY_FONT_SIZE, index);

        // Update button states
        updateFontButtons(index);
    }

    // Apply theme
    function applyTheme(themeName) {
        var theme = THEMES[themeName];
        if (!theme) return;

        var book = document.querySelector('.book');
        if (book) {
            // Remove all theme classes
            book.classList.remove('theme-white', 'theme-sepia', 'theme-night');
            // Add new theme class
            book.classList.add('theme-' + themeName);
        }

        // Force table styles for night theme (overrides custom CSS)
        applyTableStyles(themeName);

        localStorage.setItem(STORAGE_KEY_THEME, themeName);

        // Update button states
        updateThemeButtons(themeName);
    }

    // Apply inline styles to override custom CSS for theme compatibility
    function applyTableStyles(themeName) {
        var tables = document.querySelectorAll('.markdown-section table');
        var headings = document.querySelectorAll('.markdown-section h1, .markdown-section h2, .markdown-section h3, .markdown-section h4, .markdown-section h5, .markdown-section h6');

        // Apply table styles
        tables.forEach(function(table) {
            var ths = table.querySelectorAll('th');
            var tds = table.querySelectorAll('td');

            if (themeName === 'night') {
                // Night theme - apply dark styles
                table.style.background = '#1c1c1c';
                ths.forEach(function(th) {
                    th.style.backgroundColor = '#2d2d2d';
                    th.style.color = '#e8e8e8';
                    th.style.borderColor = '#3a3a3a';
                });
                tds.forEach(function(td) {
                    td.style.backgroundColor = '#1c1c1c';
                    td.style.color = '#c8c8c8';
                    td.style.borderColor = '#3a3a3a';
                });
            } else if (themeName === 'sepia') {
                // Sepia theme
                table.style.background = '';
                ths.forEach(function(th) {
                    th.style.backgroundColor = '#ebe4d0';
                    th.style.color = '#5f4b32';
                    th.style.borderColor = '#d4c9b0';
                });
                tds.forEach(function(td) {
                    td.style.backgroundColor = '';
                    td.style.color = '#5f4b32';
                    td.style.borderColor = '#d4c9b0';
                });
            } else {
                // White theme - clear inline styles (use CSS defaults)
                table.style.background = '';
                ths.forEach(function(th) {
                    th.style.backgroundColor = '';
                    th.style.color = '';
                    th.style.borderColor = '';
                });
                tds.forEach(function(td) {
                    td.style.backgroundColor = '';
                    td.style.color = '';
                    td.style.borderColor = '';
                });
            }
        });

        // Apply heading styles (h1-h6)
        headings.forEach(function(heading) {
            if (themeName === 'night') {
                // Night theme - dark background for headings
                heading.style.backgroundColor = '#2d2d2d';
                heading.style.color = '#e8e8e8';
                heading.style.borderColor = '#3a3a3a';
                heading.style.paddingLeft = '12px';
                heading.style.paddingRight = '12px';
            } else if (themeName === 'sepia') {
                // Sepia theme
                heading.style.backgroundColor = '#ebe4d0';
                heading.style.color = '#5f4b32';
                heading.style.borderColor = '#d4c9b0';
                heading.style.paddingLeft = '12px';
                heading.style.paddingRight = '12px';
            } else {
                // White theme - clear inline styles
                heading.style.backgroundColor = '';
                heading.style.color = '';
                heading.style.borderColor = '';
                heading.style.paddingLeft = '';
                heading.style.paddingRight = '';
            }
        });
    }

    // Update font size button states
    function updateFontButtons(index) {
        var decreaseBtn = document.querySelector('.fontsettings-decrease');
        var increaseBtn = document.querySelector('.fontsettings-increase');

        if (decreaseBtn) {
            decreaseBtn.disabled = (index <= 0);
            decreaseBtn.classList.toggle('disabled', index <= 0);
        }
        if (increaseBtn) {
            increaseBtn.disabled = (index >= FONT_SIZES.length - 1);
            increaseBtn.classList.toggle('disabled', index >= FONT_SIZES.length - 1);
        }
    }

    // Update theme button states
    function updateThemeButtons(themeName) {
        document.querySelectorAll('.fontsettings-theme').forEach(function(btn) {
            var btnTheme = btn.getAttribute('data-theme');
            btn.classList.toggle('active', btnTheme === themeName);
        });
    }

    // Decrease font size
    function decreaseFontSize() {
        var index = getFontSizeIndex();
        if (index > 0) {
            applyFontSize(index - 1);
        }
    }

    // Increase font size
    function increaseFontSize() {
        var index = getFontSizeIndex();
        if (index < FONT_SIZES.length - 1) {
            applyFontSize(index + 1);
        }
    }

    // Initialize fontsettings
    function init() {
        // Apply saved settings
        applyFontSize(getFontSizeIndex());
        applyTheme(getTheme());

        // Set up event listeners
        var toolbar = document.querySelector('.fontsettings-toolbar');
        if (!toolbar) return;

        // Font size buttons
        var decreaseBtn = toolbar.querySelector('.fontsettings-decrease');
        var increaseBtn = toolbar.querySelector('.fontsettings-increase');

        if (decreaseBtn) {
            decreaseBtn.addEventListener('click', function(e) {
                e.preventDefault();
                decreaseFontSize();
            });
        }

        if (increaseBtn) {
            increaseBtn.addEventListener('click', function(e) {
                e.preventDefault();
                increaseFontSize();
            });
        }

        // Theme buttons
        toolbar.querySelectorAll('.fontsettings-theme').forEach(function(btn) {
            btn.addEventListener('click', function(e) {
                e.preventDefault();
                var theme = this.getAttribute('data-theme');
                if (theme) {
                    applyTheme(theme);
                }
            });
        });
    }

    // Re-apply settings after SPA navigation
    function reapplySettings() {
        var markdownSection = document.querySelector('.markdown-section');
        if (markdownSection) {
            var size = FONT_SIZES[getFontSizeIndex()];
            markdownSection.style.fontSize = size + 'px';
        }
        // Re-apply table styles for current theme
        applyTableStyles(getTheme());
    }

    // Initialize on DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // Export for SPA navigation re-initialization
    window.guidebookFontsettings = {
        init: init,
        reapply: reapplySettings,
        applyFontSize: applyFontSize,
        applyTheme: applyTheme
    };

})();
