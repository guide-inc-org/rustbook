// GitBook-compatible JavaScript

(function() {
    'use strict';

    // Back to top button
    var backToTop = document.querySelector('.back-to-top');
    if (backToTop) {
        window.addEventListener('scroll', function() {
            if (window.scrollY > 300) {
                backToTop.classList.add('visible');
            } else {
                backToTop.classList.remove('visible');
            }
        });

        backToTop.addEventListener('click', function(e) {
            e.preventDefault();
            window.scrollTo({ top: 0, behavior: 'smooth' });
        });
    }

    // Sidebar toggle
    var sidebarToggle = document.querySelector('.sidebar-toggle');
    var book = document.querySelector('.book');
    if (sidebarToggle && book) {
        // Restore sidebar state from localStorage
        var sidebarHidden = localStorage.getItem('rustbook-sidebar-hidden') === 'true';
        if (sidebarHidden) {
            book.classList.add('sidebar-hidden');
        }

        sidebarToggle.addEventListener('click', function() {
            book.classList.add('sidebar-toggling');
            book.classList.toggle('sidebar-hidden');
            var isHidden = book.classList.contains('sidebar-hidden');
            localStorage.setItem('rustbook-sidebar-hidden', isHidden);
            setTimeout(function() {
                book.classList.remove('sidebar-toggling');
            }, 350);
        });
    }

    // Mobile menu toggle (legacy)
    var menuToggle = document.querySelector('.menu-toggle');
    var bookSummary = document.querySelector('.book-summary');
    if (menuToggle && bookSummary) {
        menuToggle.addEventListener('click', function() {
            bookSummary.classList.toggle('open');
        });
    }

    // Smooth scroll for anchor links
    document.querySelectorAll('a[href*="#"]').forEach(function(anchor) {
        anchor.addEventListener('click', function(e) {
            var href = this.getAttribute('href');
            var hashIndex = href.indexOf('#');
            if (hashIndex === -1) return;

            var hash = href.substring(hashIndex + 1);
            // Decode URL-encoded anchor (e.g., %E3%83%87%E3%82%B6%E3%82%A4%E3%83%B3 -> デザイン)
            try {
                hash = decodeURIComponent(hash);
            } catch (ex) {
                // If decoding fails, use as-is
            }

            var target = document.getElementById(hash);
            if (target) {
                e.preventDefault();
                target.scrollIntoView({ behavior: 'smooth' });
                // Update URL hash without triggering navigation
                history.pushState(null, '', '#' + encodeURIComponent(hash));
            }
        });
    });

    // SPA-like navigation for sidebar links
    // Store base URL on initial page load (e.g., /jp/)
    var baseUrl = (function() {
        var base = document.querySelector('base');
        if (base && base.href) {
            return base.href;
        }
        return window.location.href.replace(/[^/]*$/, '');
    })();

    function setupSpaNavigation() {
        var sidebar = document.querySelector('.book-summary');
        if (!sidebar) return;

        sidebar.addEventListener('click', function(e) {
            var link = e.target.closest('a');
            if (!link) return;

            var href = link.getAttribute('href');
            if (!href || href.startsWith('#') || href.startsWith('http')) return;

            e.preventDefault();
            loadPage(href, link);
        });
    }

    function loadPage(url, clickedLink) {
        // Always resolve relative to the fixed base URL
        var absoluteUrl = new URL(url, baseUrl).href;

        // Extract hash from URL if present
        var hashIndex = url.indexOf('#');
        var hash = hashIndex !== -1 ? url.substring(hashIndex + 1) : null;

        fetch(absoluteUrl)
            .then(function(response) {
                if (!response.ok) throw new Error('Page not found');
                return response.text();
            })
            .then(function(html) {
                var parser = new DOMParser();
                var doc = parser.parseFromString(html, 'text/html');

                // Update content
                var newContent = doc.querySelector('.markdown-section');
                var currentContent = document.querySelector('.markdown-section');
                if (newContent && currentContent) {
                    currentContent.innerHTML = newContent.innerHTML;
                }

                // Update title
                var newTitle = doc.querySelector('title');
                if (newTitle) {
                    document.title = newTitle.textContent;
                }

                // Update active state in sidebar
                document.querySelectorAll('.book-summary .chapter').forEach(function(ch) {
                    ch.classList.remove('active');
                });
                if (clickedLink) {
                    var chapter = clickedLink.closest('.chapter');
                    if (chapter) chapter.classList.add('active');
                }

                // Update URL
                history.pushState(null, '', url);

                // Scroll to hash anchor or top
                if (hash) {
                    try {
                        var decodedHash = decodeURIComponent(hash);
                        var target = document.getElementById(decodedHash);
                        if (target) {
                            setTimeout(function() {
                                target.scrollIntoView({ behavior: 'auto' });
                            }, 50);
                        } else {
                            window.scrollTo(0, 0);
                        }
                    } catch (ex) {
                        window.scrollTo(0, 0);
                    }
                } else {
                    window.scrollTo(0, 0);
                }

                // Re-init mermaid if present
                if (typeof mermaid !== 'undefined') {
                    mermaid.init(undefined, '.markdown-section .mermaid');
                }
            })
            .catch(function(err) {
                console.error('Navigation error:', err);
                window.location.href = url;
            });
    }

    // Handle browser back/forward
    window.addEventListener('popstate', function() {
        loadPage(location.pathname + location.hash, null);
    });

    setupSpaNavigation();

    // Handle initial page load with hash anchor
    function scrollToHashOnLoad() {
        if (!window.location.hash) return;

        var hash = window.location.hash.substring(1);
        // Decode URL-encoded anchor
        try {
            hash = decodeURIComponent(hash);
        } catch (ex) {
            // If decoding fails, use as-is
        }

        var target = document.getElementById(hash);
        if (target) {
            // Use setTimeout to ensure layout is complete after all resources load
            setTimeout(function() {
                target.scrollIntoView({ behavior: 'auto' });
            }, 100);
        }
    }

    // Use 'load' event to ensure all resources (images, CSS) are loaded
    if (document.readyState === 'complete') {
        scrollToHashOnLoad();
    } else {
        window.addEventListener('load', scrollToHashOnLoad);
    }
})();
