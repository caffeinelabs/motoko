persistent actor {
  type TemperatureSeries = [Float];

  class Weather(temperatures : TemperatureSeries) {
    public func averageTemperature() : Float {
      var sum = 0.0;
      var count = 0.0;
      for (value in temperatures.values()) {
        sum += value;
        count += 1;
      };
      return sum / count;
    };
  };

  var temperatures : TemperatureSeries = [30.0, 31.5, 29.2];
  transient var weather = Weather(temperatures);
};
