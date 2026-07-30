import unittest
from pathlib import Path

from validate_source_provenance import (
    ProvenanceError,
    validate_document,
    validate_repository,
)


REPOSITORY = Path(__file__).resolve().parent.parent
COMMIT = "a" * 40
PROTOCOL_REPOSITORY = "../remote-protocol.git"


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
        validate_document(document(), COMMIT, PROTOCOL_REPOSITORY)

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
                    validate_document(
                        provenance,
                        actual_commit,
                        PROTOCOL_REPOSITORY,
                    )

    def test_rejects_nonportable_or_drifted_submodule_repository(self) -> None:
        for actual_repository in (
            "https://github.com/example/remote-protocol",
            "../remote-protocol",
            "../../remote-protocol.git",
            "../..git",
            "../other-protocol.git",
        ):
            with self.subTest(actual_repository=actual_repository):
                with self.assertRaises(ProvenanceError):
                    validate_document(
                        document(),
                        COMMIT,
                        actual_repository,
                    )


if __name__ == "__main__":
    unittest.main()
