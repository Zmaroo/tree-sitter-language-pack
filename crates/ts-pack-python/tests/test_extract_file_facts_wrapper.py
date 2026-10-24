import unittest

import tree_sitter_language_pack as ts


class ExtractFileFactsWrapperTests(unittest.TestCase):
    def test_fetch_methods_belong_to_their_own_call(self):
        source = '\n'.join([
            'fetch("/api/leases", {headers: {Authorization: token}});',
            'fetch("/api/units", {method: "POST"});',
            'fetch("/api/read");',
            'fetch("/api/dynamic", options);',
            'fetch("/api/computed", {method: selectedMethod});',
            'fetch("/api/quoted", {"method": "PATCH"});',
            'fetch("/api/spread", {method: "POST", ...options});',
            'const unrelated = {method: "DELETE"};',
        ])
        for language in ("javascript", "typescript", "tsx"):
            with self.subTest(language=language):
                ts.get_parser(language)
                facts = ts.extract_file_facts(source, language)
                self.assertEqual(
                    {item["path"]: item["method"] for item in facts["http_calls"]},
                    {"/api/leases": "GET", "/api/units": "POST", "/api/read": "GET",
                     "/api/dynamic": "ANY", "/api/computed": "ANY",
                     "/api/quoted": "PATCH", "/api/spread": "ANY"},
                )

    def test_extract_file_facts_uses_language_then_file_path(self):
        if not ts.has_language("typescript"):
            self.skipTest("typescript parser unavailable in test environment")

        facts = ts.extract_file_facts(
            """
            async function api(path) {
                return fetch(path);
            }

            await api("/api/leases");
            """,
            "typescript",
            "src/public/assets/app.js",
        )

        self.assertIn(
            {"client": "fetch", "method": "GET", "path": "/api/leases"},
            facts.get("http_calls", []),
        )


if __name__ == "__main__":
    unittest.main()
