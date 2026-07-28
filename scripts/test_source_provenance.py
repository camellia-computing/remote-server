import unittest
from pathlib import Path

from validate_source_provenance import (
    PROTOCOL_REPOSITORY,
    ProvenanceError,
    validate_document,
    validate_repository,
)


REPOSITORY = Path(__file__).resolve().parent.parent
COMMIT = "2daff94dc8d4dae97b04ff47563f70842c47e28b"


def document(commit: object = COMMIT) -> dict[str, object]:
    return {
        "schemaVersion": 1,
        "protocol": {
            "repository": PROTOCOL_REPOSITORY,
            "commit": commit,
        },
    }


class SourceProvenanceTests(unittest.TestCase):
    def test_repository_matches_checked_out_protocol(self) -> None:
        validate_repository(REPOSITORY)

    def test_accepts_exact_canonical_source(self) -> None:
        validate_document(document(), COMMIT)

    def test_rejects_malformed_or_stale_source(self) -> None:
        cases = (
            ({**document(), "schemaVersion": 2}, COMMIT),
            (
                {
                    **document(),
                    "protocol": {
                        "repository": "https://example.invalid/protocol",
                        "commit": COMMIT,
                    },
                },
                COMMIT,
            ),
            (document("2daff94"), COMMIT),
            (document(), "0" * 40),
        )
        for provenance, actual_commit in cases:
            with self.subTest(provenance=provenance, actual_commit=actual_commit):
                with self.assertRaises(ProvenanceError):
                    validate_document(provenance, actual_commit)


if __name__ == "__main__":
    unittest.main()
