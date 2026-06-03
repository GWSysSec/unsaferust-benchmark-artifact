use metrics::GaugeValue;

#[test]
fn test_gauge_value_update_value_basic_operations() {

    let gauge = GaugeValue::Increment(0.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, 10.0, "Incrementing 0.0 from 10.0 should yield 10.0");


    let gauge = GaugeValue::Increment(5.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, 15.0, "Incrementing 5.0 from 10.0 should yield 15.0");


    let gauge = GaugeValue::Decrement(3.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, 7.0, "Decrementing 3.0 from 10.0 should yield 7.0");


    let gauge = GaugeValue::Absolute(42.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, 42.0, "Absolute 42.0 should ignore input and yield 42.0");


    let gauge = GaugeValue::Absolute(99.0);
    let result = gauge.update_value(0.0);
    assert_eq!(result, 99.0, "Absolute 99.0 should yield 99.0 regardless of input");


    let gauge = GaugeValue::Decrement(15.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, -5.0, "Decrementing 15.0 from 10.0 should yield -5.0");


    let gauge = GaugeValue::Increment(-3.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, 7.0, "Incrementing -3.0 from 10.0 should yield 7.0");


    let gauge = GaugeValue::Decrement(-3.0);
    let result = gauge.update_value(10.0);
    assert_eq!(result, 13.0, "Decrementing -3.0 from 10.0 should yield 13.0");
}

#[test]
fn test_gauge_value_update_value_zero_boundary() {

    let gauge = GaugeValue::Increment(0.0);
    let result = gauge.update_value(55.5);
    assert_eq!(result, 55.5, "Incrementing by 0 should not change value");


    let gauge = GaugeValue::Decrement(0.0);
    let result = gauge.update_value(55.5);
    assert_eq!(result, 55.5, "Decrementing by 0 should not change value");


    let gauge = GaugeValue::Absolute(0.0);
    let result = gauge.update_value(55.5);
    assert_eq!(result, 0.0, "Absolute 0.0 should set value to 0.0");


    let gauge = GaugeValue::Increment(7.7);
    let result = gauge.update_value(0.0);
    assert_eq!(result, 7.7, "Incrementing 7.7 from 0.0 should yield 7.7");

    let gauge = GaugeValue::Decrement(7.7);
    let result = gauge.update_value(0.0);
    assert_eq!(result, -7.7, "Decrementing 7.7 from 0.0 should yield -7.7");


    let gauge = GaugeValue::Increment(0.0);
    let result = gauge.update_value(0.0);
    assert_eq!(result, 0.0, "Incrementing 0.0 from 0.0 should yield 0.0");

    let gauge = GaugeValue::Decrement(0.0);
    let result = gauge.update_value(0.0);
    assert_eq!(result, 0.0, "Decrementing 0.0 from 0.0 should yield 0.0");

    let gauge = GaugeValue::Absolute(0.0);
    let result = gauge.update_value(0.0);
    assert_eq!(result, 0.0, "Absolute 0.0 from 0.0 should yield 0.0");
}

#[test]
fn test_gauge_value_update_value_large_values() {
    let large = f64::MAX / 2.0;


    let gauge = GaugeValue::Increment(large);
    let result = gauge.update_value(0.0);
    assert_eq!(result, large, "Large increment from 0 should yield the large value");


    let gauge = GaugeValue::Absolute(large);
    let result = gauge.update_value(1.0);
    assert_eq!(result, large, "Absolute large value should set to large");


    let gauge = GaugeValue::Decrement(large);
    let result = gauge.update_value(0.0);
    assert_eq!(result, -large, "Large decrement from 0 should yield negative large");


    let gauge = GaugeValue::Increment(1.0);
    let result = gauge.update_value(-large);
    assert_eq!(result, -large + 1.0, "Increment 1.0 from -large should yield -large + 1.0");


    let small = f64::MIN_POSITIVE;
    let gauge = GaugeValue::Increment(small);
    let result = gauge.update_value(0.0);
    assert_eq!(result, small, "Increment by MIN_POSITIVE from 0 should yield MIN_POSITIVE");

    let gauge = GaugeValue::Decrement(small);
    let result = gauge.update_value(small);
    assert_eq!(result, 0.0, "Decrement MIN_POSITIVE from MIN_POSITIVE should yield 0.0");

    let gauge = GaugeValue::Absolute(small);
    let result = gauge.update_value(large);
    assert_eq!(result, small, "Absolute MIN_POSITIVE should override large input");


    let gauge = GaugeValue::Absolute(-large);
    let result = gauge.update_value(large);
    assert_eq!(result, -large, "Absolute -large should override positive large input");
}

#[test]
fn test_gauge_value_update_value_sequential_workflow() {

    let mut current = 0.0_f64;


    let op = GaugeValue::Absolute(100.0);
    current = op.update_value(current);
    assert_eq!(current, 100.0, "After absolute set, value should be 100.0");


    let op = GaugeValue::Increment(25.0);
    current = op.update_value(current);
    assert_eq!(current, 125.0, "After increment 25, value should be 125.0");


    let op = GaugeValue::Decrement(50.0);
    current = op.update_value(current);
    assert_eq!(current, 75.0, "After decrement 50, value should be 75.0");


    let op = GaugeValue::Increment(0.5);
    current = op.update_value(current);
    assert_eq!(current, 75.5, "After increment 0.5, value should be 75.5");


    let op = GaugeValue::Absolute(0.0);
    current = op.update_value(current);
    assert_eq!(current, 0.0, "After absolute 0, value should be 0.0");


    let op = GaugeValue::Decrement(10.0);
    current = op.update_value(current);
    assert_eq!(current, -10.0, "After decrement 10 from 0, value should be -10.0");


    let op = GaugeValue::Increment(10.0);
    current = op.update_value(current);
    assert_eq!(current, 0.0, "After increment 10 from -10, value should be 0.0");


    let op = GaugeValue::Absolute(-999.0);
    current = op.update_value(current);
    assert_eq!(current, -999.0, "After absolute -999, value should be -999.0");
}

#[test]
fn test_gauge_value_update_value_fractional_precision() {

    let gauge = GaugeValue::Increment(0.1);
    let result = gauge.update_value(0.2);

    assert!((result - 0.3).abs() < 1e-15, "0.1 + 0.2 should be approximately 0.3");

    let gauge = GaugeValue::Decrement(0.1);
    let result = gauge.update_value(0.3);
    assert!((result - 0.2).abs() < 1e-15, "0.3 - 0.1 should be approximately 0.2");

    let gauge = GaugeValue::Absolute(std::f64::consts::PI);
    let result = gauge.update_value(0.0);
    assert_eq!(result, std::f64::consts::PI, "Absolute PI should yield PI");

    let gauge = GaugeValue::Increment(std::f64::consts::E);
    let result = gauge.update_value(std::f64::consts::PI);
    let expected = std::f64::consts::PI + std::f64::consts::E;
    assert_eq!(result, expected, "PI + E should be exact in f64 addition");


    let mut current = 0.0_f64;
    for _ in 0..10 {
        let op = GaugeValue::Increment(1.0);
        current = op.update_value(current);
    }
    assert_eq!(current, 10.0, "10 increments of 1.0 from 0 should yield 10.0");


    for _ in 0..5 {
        let op = GaugeValue::Decrement(2.0);
        current = op.update_value(current);
    }
    assert_eq!(current, 0.0, "5 decrements of 2.0 from 10 should yield 0.0");


    let op = GaugeValue::Absolute(1.23456789);
    let result = op.update_value(current);
    assert_eq!(result, 1.23456789, "Absolute should set exact value");


    let result2 = op.update_value(999999.0);
    assert_eq!(result, result2, "Absolute should yield same result regardless of input");
}