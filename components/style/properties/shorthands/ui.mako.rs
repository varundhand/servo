/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

<%namespace name="helpers" file="/helpers.mako.rs" />

macro_rules! try_parse_one {
    ($context: expr, $input: expr, $var: ident, $prop_module: ident) => {
        if $var.is_none() {
            if let Ok(value) = $input.try_parse(|i| {
                $prop_module::single_value::parse($context, i)
            }) {
                $var = Some(value);
                continue;
            }
        }
    };
}

<%helpers:shorthand name="transition"
                    engines="gecko servo"
                    extra_prefixes="moz:layout.css.prefixes.transitions webkit"
                    sub_properties="transition-property transition-duration
                                    transition-timing-function
                                    transition-delay"
                    spec="https://drafts.csswg.org/css-transitions/#propdef-transition">
    use crate::parser::Parse;
    % for prop in "delay duration property timing_function".split():
    use crate::properties::longhands::transition_${prop};
    % endfor
    use crate::values::specified::TransitionProperty;

    pub fn parse_value<'i, 't>(
        context: &ParserContext,
        input: &mut Parser<'i, 't>,
    ) -> Result<Longhands, ParseError<'i>> {
        struct SingleTransition {
            % for prop in "duration timing_function delay".split():
            transition_${prop}: transition_${prop}::SingleSpecifiedValue,
            % endfor
            // Unlike other properties, transition-property uses an Option<> to
            // represent 'none' as `None`.
            transition_property: Option<TransitionProperty>,
        }

        fn parse_one_transition<'i, 't>(
            context: &ParserContext,
            input: &mut Parser<'i, 't>,
        ) -> Result<SingleTransition,ParseError<'i>> {
            % for prop in "property duration timing_function delay".split():
            let mut ${prop} = None;
            % endfor

            let mut parsed = 0;
            loop {
                parsed += 1;

                try_parse_one!(context, input, duration, transition_duration);
                try_parse_one!(context, input, timing_function, transition_timing_function);
                try_parse_one!(context, input, delay, transition_delay);
                // Must check 'transition-property' after 'transition-timing-function' since
                // 'transition-property' accepts any keyword.
                if property.is_none() {
                    if let Ok(value) = input.try_parse(|i| TransitionProperty::parse(context, i)) {
                        property = Some(Some(value));
                        continue;
                    }

                    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
                        // 'none' is not a valid value for <single-transition-property>,
                        // so it's not acceptable in the function above.
                        property = Some(None);
                        continue;
                    }
                }

                parsed -= 1;
                break
            }

            if parsed != 0 {
                Ok(SingleTransition {
                    % for prop in "duration timing_function delay".split():
                    transition_${prop}: ${prop}.unwrap_or_else(transition_${prop}::single_value
                                                                                 ::get_initial_specified_value),
                    % endfor
                    transition_property: property.unwrap_or(
                        Some(transition_property::single_value::get_initial_specified_value())),
                })
            } else {
                Err(input.new_custom_error(StyleParseErrorKind::UnspecifiedError))
            }
        }

        % for prop in "property duration timing_function delay".split():
        let mut ${prop}s = Vec::new();
        % endfor

        let results = input.parse_comma_separated(|i| parse_one_transition(context, i))?;
        let multiple_items = results.len() >= 2;
        for result in results {
            if let Some(value) = result.transition_property {
                propertys.push(value);
            } else if multiple_items {
                // If there is more than one item, and any of transitions has 'none',
                // then it's invalid. Othersize, leave propertys to be empty (which
                // means "transition-property: none");
                return Err(input.new_custom_error(StyleParseErrorKind::UnspecifiedError));
            }

            % for prop in "duration timing_function delay".split():
            ${prop}s.push(result.transition_${prop});
            % endfor
        }

        Ok(expanded! {
            % for prop in "property duration timing_function delay".split():
            transition_${prop}: transition_${prop}::SpecifiedValue(${prop}s.into()),
            % endfor
        })
    }

    impl<'a> ToCss for LonghandsToSerialize<'a>  {
        fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result where W: fmt::Write {
            let property_len = self.transition_property.0.len();

            // There are two cases that we can do shorthand serialization:
            // * when all value lists have the same length, or
            // * when transition-property is none, and other value lists have exactly one item.
            if property_len == 0 {
                % for name in "duration delay timing_function".split():
                    if self.transition_${name}.0.len() != 1 {
                        return Ok(());
                    }
                % endfor
            } else {
                % for name in "duration delay timing_function".split():
                    if self.transition_${name}.0.len() != property_len {
                        return Ok(());
                    }
                % endfor
            }

            // Representative length.
            let len = self.transition_duration.0.len();

            for i in 0..len {
                if i != 0 {
                    dest.write_str(", ")?;
                }
                if property_len == 0 {
                    dest.write_str("none")?;
                } else {
                    self.transition_property.0[i].to_css(dest)?;
                }
                % for name in "duration timing_function delay".split():
                    dest.write_str(" ")?;
                    self.transition_${name}.0[i].to_css(dest)?;
                % endfor
            }
            Ok(())
        }
    }
</%helpers:shorthand>

<%helpers:shorthand name="animation"
                    engines="gecko servo"
                    extra_prefixes="moz:layout.css.prefixes.animations webkit"
                    sub_properties="animation-name animation-duration
                                    animation-timing-function animation-delay
                                    animation-iteration-count animation-direction
                                    animation-fill-mode animation-play-state animation-timeline"
                    rule_types_allowed="Style"
                    spec="https://drafts.csswg.org/css-animations/#propdef-animation">
    <%
        props = "name timeline duration timing_function delay iteration_count \
                 direction fill_mode play_state".split()
    %>
    % for prop in props:
    use crate::properties::longhands::animation_${prop};
    % endfor

    pub fn parse_value<'i, 't>(
        context: &ParserContext,
        input: &mut Parser<'i, 't>,
    ) -> Result<Longhands, ParseError<'i>> {
        struct SingleAnimation {
            % for prop in props:
            animation_${prop}: animation_${prop}::SingleSpecifiedValue,
            % endfor
        }

        fn scroll_linked_animations_enabled() -> bool {
            #[cfg(feature = "gecko")]
            return static_prefs::pref!("layout.css.scroll-linked-animations.enabled");
            #[cfg(feature = "servo")]
            return false;
        }

        fn parse_one_animation<'i, 't>(
            context: &ParserContext,
            input: &mut Parser<'i, 't>,
        ) -> Result<SingleAnimation, ParseError<'i>> {
            % for prop in props:
            let mut ${prop} = None;
            % endfor

            let mut parsed = 0;
            // NB: Name must be the last one here so that keywords valid for other
            // longhands are not interpreted as names.
            //
            // Also, duration must be before delay, see
            // https://drafts.csswg.org/css-animations/#typedef-single-animation
            loop {
                parsed += 1;
                try_parse_one!(context, input, duration, animation_duration);
                try_parse_one!(context, input, timing_function, animation_timing_function);
                try_parse_one!(context, input, delay, animation_delay);
                try_parse_one!(context, input, iteration_count, animation_iteration_count);
                try_parse_one!(context, input, direction, animation_direction);
                try_parse_one!(context, input, fill_mode, animation_fill_mode);
                try_parse_one!(context, input, play_state, animation_play_state);
                try_parse_one!(context, input, name, animation_name);
                if scroll_linked_animations_enabled() {
                    try_parse_one!(context, input, timeline, animation_timeline);
                }

                parsed -= 1;
                break
            }

            // If nothing is parsed, this is an invalid entry.
            if parsed == 0 {
                Err(input.new_custom_error(StyleParseErrorKind::UnspecifiedError))
            } else {
                Ok(SingleAnimation {
                    % for prop in props:
                    animation_${prop}: ${prop}.unwrap_or_else(animation_${prop}::single_value
                                                              ::get_initial_specified_value),
                    % endfor
                })
            }
        }

        % for prop in props:
        let mut ${prop}s = vec![];
        % endfor

        let results = input.parse_comma_separated(|i| parse_one_animation(context, i))?;
        for result in results.into_iter() {
            % for prop in props:
            ${prop}s.push(result.animation_${prop});
            % endfor
        }

        Ok(expanded! {
            % for prop in props:
            animation_${prop}: animation_${prop}::SpecifiedValue(${prop}s.into()),
            % endfor
        })
    }

    impl<'a> ToCss for LonghandsToSerialize<'a>  {
        fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result where W: fmt::Write {
            let len = self.animation_name.0.len();
            // There should be at least one declared value
            if len == 0 {
                return Ok(());
            }

            // If any value list length is differs then we don't do a shorthand serialization
            // either.
            % for name in props[2:]:
                if len != self.animation_${name}.0.len() {
                    return Ok(())
                }
            % endfor

            // If the preference of animation-timeline is disabled, `self.animation_timeline` is
            // None.
            if self.animation_timeline.map_or(false, |v| len != v.0.len()) {
                return Ok(());
            }

            for i in 0..len {
                if i != 0 {
                    dest.write_str(", ")?;
                }

                % for name in props[2:]:
                    self.animation_${name}.0[i].to_css(dest)?;
                    dest.write_str(" ")?;
                % endfor

                self.animation_name.0[i].to_css(dest)?;

                // Based on the spec, the default values of other properties must be output in at
                // least the cases necessary to distinguish an animation-name. The serialization
                // order of animation-timeline is always later than animation-name, so it's fine
                // to not serialize it if it is the default value. It's still possible to
                // distinguish them (because we always serialize animation-name).
                // https://drafts.csswg.org/css-animations-1/#animation
                // https://drafts.csswg.org/css-animations-2/#typedef-single-animation
                //
                // Note: it's also fine to always serialize this. However, it seems Blink
                // doesn't serialize default animation-timeline now, so we follow the same rule.
                if let Some(ref timeline) = self.animation_timeline {
                    if !timeline.0[i].is_auto() {
                        dest.write_char(' ')?;
                        timeline.0[i].to_css(dest)?;
                    }
                }
            }
            Ok(())
        }
    }
</%helpers:shorthand>

<%helpers:shorthand
    engines="gecko"
    name="scroll-timeline"
    sub_properties="scroll-timeline-name scroll-timeline-axis"
    gecko_pref="layout.css.scroll-linked-animations.enabled",
    // Also in https://drafts.csswg.org/scroll-animations-1/rewrite#scroll-timeline-shorthand
    spec="https://github.com/w3c/csswg-drafts/issues/6674"
>
    pub fn parse_value<'i>(
        context: &ParserContext,
        input: &mut Parser<'i, '_>,
    ) -> Result<Longhands, ParseError<'i>> {
        use crate::parser::Parse;
        use crate::values::specified::box_::{ScrollAxis, ScrollTimelineName};

        let mut name = None;
        let mut axis = None;
        loop {
            // Note: When parsing positionally-ambiguous keywords in a property value, a
            // <custom-ident> production can only claim the keyword if no other unfulfilled
            // production can claim it. So we try to parse `scroll-timeline-axis` first.
            //
            // https://drafts.csswg.org/css-values-4/#custom-idents

            if axis.is_none() {
                axis = input.try_parse(ScrollAxis::parse).ok();
            }

            if name.is_none() {
                if let Ok(value) = input.try_parse(|i| ScrollTimelineName::parse(context, i)) {
                    name = Some(value);
                    continue;
                }
            }
            break;
        }

        // Must occur one or more.
        if name.is_none() && axis.is_none() {
            return Err(input.new_custom_error(StyleParseErrorKind::UnspecifiedError));
        }

        Ok(expanded! {
            scroll_timeline_name: name.unwrap_or(ScrollTimelineName::none()),
            scroll_timeline_axis: axis.unwrap_or_default(),
        })
    }

    impl<'a> ToCss for LonghandsToSerialize<'a>  {
        fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result where W: fmt::Write {
            use crate::values::specified::box_::ScrollAxis;

            let is_default_axis = self.scroll_timeline_axis == &ScrollAxis::default();
            let is_default_name = self.scroll_timeline_name.0.is_none();

            // Note: if both are default values, we serialize the default axis (because it is the
            // first value per spec).
            if !is_default_axis || (is_default_axis && is_default_name) {
                self.scroll_timeline_axis.to_css(dest)?;
            }

            if !is_default_name {
                if !is_default_axis {
                    dest.write_char(' ')?;
                }
                self.scroll_timeline_name.to_css(dest)?;
            }

            Ok(())
        }
    }
</%helpers:shorthand>
