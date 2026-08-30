use laruche_essaim::reine_juge::parser_scorecard;

/// Observed with deepseek-v4-flash: the judge preambles in prose, hits the token
/// ceiling and never reaches the scorecard. With ANALYSIS first that lost EVERYTHING.
/// Scores first means a cut answer still carries a usable verdict.
#[test]
fn une_reponse_coupee_apres_les_scores_reste_exploitable() {
    let coupee = "RELEVANCE: 85\nMETHODOLOGY: 80\nOBJECTIVE: 82\nBRAND: 90\n\
                  CONFIDENCE: 88\nVERDICT: approve\nINSTRUCTION: \nREASON: on scope.\n\
                  ANALYSIS: Clear and on-scope; the tone is warm which is fine, and the claims are grou";
    let c = parser_scorecard(coupee).expect("verdict perdu alors que les scores etaient la");
    assert_eq!(c.pertinence, 85);
    assert_eq!(c.methodologie, 80);
    assert_eq!(c.confiance, 88);
}

/// The old shape: prose first, cut before any score. Nothing to salvage - which is
/// exactly the failure this reordering removes.
#[test]
fn une_reponse_coupee_avant_les_scores_est_bien_rejetee() {
    let prose = "We need to assess the draft. The user asked \"Explain the project \
                 architecture\". The draft is a detailed overview. It seems thorough, but we \
                 need to check grou";
    assert!(parser_scorecard(prose).is_err());
}

/// The scorecard must parse whatever the order, so an older model that still answers
/// ANALYSIS-first is not suddenly rejected.
#[test]
fn lordre_des_lignes_reste_indifferent() {
    let ancien = "ANALYSIS: fine.\nRELEVANCE: 70\nMETHODOLOGY: 60\nOBJECTIVE: 65\n\
                  BRAND: 80\nCONFIDENCE: 75\nVERDICT: revise\nINSTRUCTION: cite the source\n\
                  REASON: ungrounded";
    let c = parser_scorecard(ancien).expect("ordre historique casse");
    assert_eq!(c.pertinence, 70);
    assert_eq!(c.instruction.trim(), "cite the source");
}
