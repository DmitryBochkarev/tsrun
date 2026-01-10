// tsrun site JavaScript

(function() {
  'use strict';

  // Copy to clipboard
  function copyCode(button) {
    const codeBlock = button.closest('.code-block');
    const code = codeBlock.querySelector('code').textContent;

    navigator.clipboard.writeText(code).then(() => {
      const originalText = button.textContent;
      button.textContent = 'Copied!';
      button.style.color = 'var(--success)';
      setTimeout(() => {
        button.textContent = originalText;
        button.style.color = '';
      }, 2000);
    }).catch(() => {
      button.textContent = 'Failed';
      setTimeout(() => {
        button.textContent = 'Copy';
      }, 2000);
    });
  }

  // Mark active nav link
  function setActiveNavLink() {
    const path = window.location.pathname;
    const links = document.querySelectorAll('.nav-links a');

    links.forEach(link => {
      link.classList.remove('active');
      const href = link.getAttribute('href');
      if (href === path || (href !== '/' && path.startsWith(href))) {
        link.classList.add('active');
      }
    });
  }

  // Tab switching
  function initTabs() {
    document.querySelectorAll('.tab').forEach(tab => {
      tab.addEventListener('click', () => {
        const tabGroup = tab.closest('.use-cases');
        if (!tabGroup) return;

        // Remove active from all tabs and content in this group
        tabGroup.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        tabGroup.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));

        // Add active to clicked tab and corresponding content
        tab.classList.add('active');
        const content = document.getElementById(tab.dataset.tab);
        if (content) {
          content.classList.add('active');
          // Re-highlight code in the newly visible tab
          if (typeof hljs !== 'undefined') {
            content.querySelectorAll('pre code').forEach(block => {
              hljs.highlightElement(block);
            });
          }
        }
      });
    });
  }

  // Initialize
  document.addEventListener('DOMContentLoaded', () => {
    // Copy buttons
    document.querySelectorAll('.copy-btn').forEach(btn => {
      btn.addEventListener('click', () => copyCode(btn));
    });

    // Active nav link
    setActiveNavLink();

    // Tabs
    initTabs();

    // Highlight.js initialization
    if (typeof hljs !== 'undefined') {
      hljs.highlightAll();
    }
  });

  // Expose for onclick handlers
  window.tsrunCopyCode = copyCode;
})();
