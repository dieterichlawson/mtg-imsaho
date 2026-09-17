// The engine's types as serde writes them.
//
// This file mirrors `mtg-engine`'s `GameView`, `LegalActions`, `Action`
// and their parts, in the JSON shape `serde_json` gives them: newtype ids
// are numbers, unit variants are strings, and a data-carrying enum
// variant is an object with one key. It is a description of what the seat
// sends, not a second schema the seat has to satisfy — a field the page
// does not know about is simply ignored, and a field it does know about
// is typed here so the compiler notices when the shape changes.

export type PlayerId = number;
export type ObjectId = number;
export type CardId = number;

export type Color = "White" | "Blue" | "Black" | "Red" | "Green";
export type ManaType = Color | "Colorless";
export type ManaSymbol = "X" | { Colored: Color } | { Generic: number } | string | Record<string, unknown>;
export interface ManaCost { symbols: ManaSymbol[] }
export interface ManaPool { mana: Partial<Record<ManaType, number>> }

export type Step = "Untap" | "Upkeep" | "Draw" | "PrecombatMain" | "BeginCombat" | "DeclareAttackers"
  | "DeclareBlockers" | "CombatDamage" | "EndCombat" | "PostcombatMain" | "EndStep" | "Cleanup";
export type CardType = "Land" | "Creature" | "Instant" | "Sorcery" | "Enchantment" | "Artifact" | "Planeswalker";
export type Supertype = "Basic" | "Legendary" | "Snow";
export type Keyword = string;
export type CounterType = string;
export type Zone = string;

export type Target = { Object: ObjectId } | { Player: PlayerId } | "Illegal";

export interface CardView {
  object_id: ObjectId;
  card_id: CardId;
  name: string;
  cost: ManaCost | null;
  supertypes: Supertype[];
  card_types: CardType[];
  power: number | null;
  toughness: number | null;
  oracle_text: string;
  owner: PlayerId;
  flashback_cost: ManaCost | null;
}

export type AttackTarget = { Player: PlayerId } | { Planeswalker: ObjectId };

export interface PermanentView {
  object_id: ObjectId;
  card_id: CardId;
  name: string;
  supertypes: Supertype[];
  card_types: CardType[];
  controller: PlayerId;
  owner: PlayerId;
  tapped: boolean;
  power: number | null;
  toughness: number | null;
  effective_power: number | null;
  effective_toughness: number | null;
  damage_marked: number;
  regeneration_shields: number;
  summoning_sick: boolean;
  attached_to: ObjectId | null;
  attached_to_player: PlayerId | null;
  keywords: Keyword[];
  colors: Color[];
  subtypes: string[];
  printed_power: number | null;
  printed_toughness: number | null;
  star_pt: boolean;
  is_token: boolean;
  attacking: AttackTarget | null;
  blocking: ObjectId[];
  blocked_by: ObjectId[];
  protections: string[];
  restrictions: string[];
  oracle_text: string;
  granted_abilities: string[];
  counters: Partial<Record<CounterType, number>>;
  loyalty_abilities: [number, string][];
  mana_abilities: [number, string][];
  named_card: string | null;
}

export interface StackItemView {
  object_id: ObjectId;
  card_id: CardId;
  name: string;
  controller: PlayerId;
  targets: Target[];
  x_value: number | null;
}

export interface OpponentView {
  id: PlayerId;
  life: number;
  hand_size: number;
  library_size: number;
  mana_pool: ManaPool;
  mulligan_count: number;
}

export interface GameView {
  you: PlayerId;
  your_hand: CardView[];
  your_life: number;
  your_mana_pool: ManaPool;
  your_library_size: number;
  your_library_cards: CardView[];
  your_mulligan_count: number;
  opponents: OpponentView[];
  battlefield: PermanentView[];
  graveyards: [PlayerId, CardView[]][];
  stack: StackItemView[];
  first_strike_damage_step: boolean;
  exile: CardView[];
  step: Step;
  active_player: PlayerId;
  priority_player: PlayerId | null;
  turn_number: number;
  display_log: string[];
  full_log: string[];
  revealed_names: Record<string, string>;
}

/** Anything the view can name: a permanent, a card in a zone, a stack item, or just a name. */
export type ViewObject = (PermanentView | CardView | StackItemView | { object_id: ObjectId; name: string })
  & Partial<PermanentView> & Partial<CardView> & Partial<StackItemView>;

// ------------------------------------------------------------- actions

export interface FundingResponse { pool: Partial<Record<ManaType, number>>; taps: Record<string, number> }

export type ResolvedChoice =
  | "CancelCast"
  | { PayDecision: boolean }
  | { YesNoDecision: boolean }
  | { ChosenTarget: Target | null }
  | { ChosenCard: ObjectId }
  | { ChosenIndex: [number, string] }
  | { ChosenOrder: number[] }
  | { ChosenSubset: ObjectId[] }
  | { XFunding: FundingResponse }
  | { ChosenExileSet: ObjectId[] }
  | { ChosenTargetSet: Target[] }
  | { ChosenObjectSet: ObjectId[] };

export interface CastSpellAction {
  object_id: ObjectId;
  targets: Target[];
  sacrifice: ObjectId | null;
  exile_count: number | null;
  exile_ids: ObjectId[];
  alternative_cost: ManaCost | null;
  tap_plan: [ObjectId, number][];
}

export interface ActivateAbilityAction {
  object_id: ObjectId;
  ability_index: number;
  targets: Target[];
  tap_plan: [ObjectId, number][];
  sacrifice: ObjectId | null;
  x_value: number | null;
  source_card_id: CardId | null;
}

export type Action =
  | "PassPriority" | "Concede" | "MulliganKeep" | "MulliganMull" | "AbandonGame"
  | { PlayLand: { object_id: ObjectId } }
  | { CastSpell: CastSpellAction }
  | { ActivateManaAbility: { object_id: ObjectId; ability_index: number } }
  | { ActivateAbility: ActivateAbilityAction }
  | { ActivateLoyaltyAbility: { object_id: ObjectId; ability_index: number; targets: Target[] } }
  | { DeclareAttackers: { attackers: [ObjectId, PlayerId][]; planeswalker_attacks: [ObjectId, ObjectId][] } }
  | { DeclareBlockers: { assignments: [ObjectId, ObjectId][] } }
  | { DiscardCards: { cards: ObjectId[] } }
  | { BottomCards: { cards: ObjectId[] } }
  | { ResolveChoice: { choice: ResolvedChoice } };

// ------------------------------------------------------------- prompts

export type CastTargetSpec =
  | "NoTargets" | "ChosenAtCast"
  | { SingleTarget: Target[] }
  | { TwoTargets: { first: Target[]; second: Target[][]; second_min: number; second_max: number } };

export interface CastableSpell {
  object_id: ObjectId;
  name: string;
  is_flashback: boolean;
  target_spec: CastTargetSpec;
  tap_plan: [ObjectId, number][];
  exile_x_from_gy_max: number | null;
  sacrifice_options: ObjectId[];
  additional_cost_label: string | null;
  alternative_cost: ManaCost | null;
  from_graveyard: boolean;
}

export interface ActivatableAbilityOption { targets: Target[]; sacrifice: ObjectId | null }

export interface ActivatableAbility {
  object_id: ObjectId;
  ability_index: number;
  source_card_id: CardId | null;
  name: string;
  description: string;
  target_options: Target[];
  tap_plan: [ObjectId, number][];
  option_combos: ActivatableAbilityOption[];
}

export type CombatPrompt =
  | { ChooseAttackers: { eligible: ObjectId[]; must_attack: ObjectId[]; defending_player: PlayerId; defending_planeswalkers?: ObjectId[] } }
  | { ChooseBlockers: { eligible_blockers: ObjectId[]; attackers: ObjectId[]; legal_blocks: Record<string, ObjectId[]>; min_blockers?: Record<string, number> } };

export interface SetPrompt {
  kind: "BottomAfterMulligan" | "DiscardToHandSize";
  player: PlayerId;
  options: ObjectId[];
  min: number;
  max: number;
}

export interface FundingGroup { name: string; source_ids: ObjectId[]; mana_per_tap: number; [k: string]: unknown }
export interface FundingOptions {
  pool: Partial<Record<ManaType, number>>;
  groups: FundingGroup[];
  max_x: number;
  x_discount?: number;
}

/**
 * One `ResolutionChoiceKind`, externally tagged: `{ ChooseTarget: {...} }`.
 * The payload is typed by the fields the page reads; every kind has a
 * `description`, and the rest are present for the kinds that carry them.
 */
export interface ResolutionPayload {
  description: string;
  options?: unknown[];
  min?: number;
  max?: number;
  permanents?: ObjectId[];
  fixed?: Target[];
  source_id?: ObjectId;
  cards?: ObjectId[];
  player?: PlayerId;
  [k: string]: unknown;
}
export type ResolutionPrompt = Record<string, ResolutionPayload>;

export interface LegalActions {
  actions: Action[];
  combat_prompt: CombatPrompt | null;
  castable_spells: CastableSpell[];
  activatable_abilities: ActivatableAbility[];
  context: string | null;
  resolution_prompt: ResolutionPrompt | null;
  set_prompt: SetPrompt | null;
}

// ------------------------------------------------------------ messages

export interface Decision { seq: number; legal: LegalActions; combat: CombatPrompt | null }

export type ServerMessage =
  | { type: "decision"; seq: number; seat: PlayerId; view: GameView; legal: LegalActions; combat: CombatPrompt | null }
  | { type: "view"; seat: PlayerId; view: GameView }
  | { type: "notice"; seq: number; text: string }
  | { type: "game_over"; seat: PlayerId; view: GameView; summary: string }
  /** Decision `seq` has been answered — by this page or another one on the
   *  same seat. Whoever still holds it must stop offering it (issue #516). */
  | { type: "answered"; seq: number }
  /** The seat's settings, which every page attached to it shares. A page
   *  that decides for the seat — auto-passing a priority — cannot decide it
   *  from settings only it can see (issue #515). */
  | { type: "settings"; stop_at_pass: boolean; auto_pass_since_turn: number | null };

export type ClientMessage =
  | { type: "hello" }
  | { type: "action"; seq: number; action: Action }
  | { type: "settings"; stop_at_pass: boolean; auto_pass_since_turn: number | null };

/** The one key of an externally tagged enum value. */
export function tag(v: object): string {
  return Object.keys(v)[0];
}
